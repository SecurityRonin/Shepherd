# Shepherd Pro — Backend Design (v2)

> Supersedes `2026-03-12-shepherd-pro-freemium-design.md` with all 8 generation endpoints,
> vision via Claude 3.5 Sonnet, and confirmed repo/domain decisions.

## Goal

Build `SecurityRonin/shepherd-pro` — a Next.js 15 backend deployed at `api.shepherd.codes`
that provides auth, credit management, Stripe payments, and 8 generative AI endpoints for
the Shepherd desktop app.

## Architecture

```
Shepherd Desktop (Tauri)
  └── CloudClient (Rust)
        │  HTTPS — Bearer JWT
        ▼
api.shepherd.codes  (Vercel — SecurityRonin/shepherd-pro)
  ├── /api/auth/           Supabase OAuth + magic link
  ├── /api/credits/        Balance + Stripe Checkout
  ├── /api/generate/       8 generation routes
  └── /api/webhooks/       Stripe payment events
        │
        ├── Supabase        Auth + Postgres + RLS
        ├── OpenRouter      logo · name · northstar · vision
        ├── Firecrawl       scrape · crawl
        ├── Exa             search
        └── Stripe          subscriptions + top-ups
```

**Repo:** `SecurityRonin/shepherd-pro` (separate from desktop app)
**Domain:** `api.shepherd.codes` (Vercel custom domain)
**Stack:** Next.js 15 App Router, TypeScript, Supabase, Stripe, Vitest + MSW

## Project Structure

```
shepherd-pro/
├── .env.example
├── next.config.ts
├── package.json
├── tsconfig.json
├── supabase/
│   ├── config.toml
│   └── migrations/
│       └── 001_initial_schema.sql
├── src/
│   ├── lib/
│   │   ├── supabase.ts          # Admin client (service role) + JWT verify
│   │   ├── openrouter.ts        # OpenRouter API client
│   │   ├── firecrawl.ts         # Firecrawl client (scrape + crawl)
│   │   ├── exa.ts               # Exa search client
│   │   ├── stripe.ts            # Stripe client + webhook helpers
│   │   ├── credits.ts           # check_trial(), deduct_credits()
│   │   └── middleware.ts        # verifyJwt, requireCredits wrappers
│   ├── app/api/
│   │   ├── auth/
│   │   │   ├── login/route.ts
│   │   │   └── callback/route.ts
│   │   ├── credits/
│   │   │   ├── balance/route.ts
│   │   │   └── purchase/route.ts
│   │   ├── generate/
│   │   │   ├── logo/route.ts
│   │   │   ├── name/route.ts
│   │   │   ├── northstar/route.ts
│   │   │   ├── scrape/route.ts
│   │   │   ├── crawl/
│   │   │   │   ├── route.ts          # POST — start crawl
│   │   │   │   └── [id]/route.ts     # GET — poll status
│   │   │   ├── vision/route.ts
│   │   │   └── search/route.ts
│   │   └── webhooks/
│   │       └── stripe/route.ts
│   └── types/index.ts
└── __tests__/
    ├── lib/credits.test.ts
    ├── lib/openrouter.test.ts
    ├── routes/auth.test.ts
    ├── routes/credits.test.ts
    ├── routes/generate/
    │   ├── logo.test.ts
    │   ├── name.test.ts
    │   ├── northstar.test.ts
    │   ├── scrape.test.ts
    │   ├── crawl.test.ts
    │   ├── crawl-status.test.ts
    │   ├── vision.test.ts
    │   └── search.test.ts
    └── routes/webhooks/stripe.test.ts
```

## Environment Variables

```bash
# OpenRouter — covers logo (Ideogram), name, northstar, vision (Claude 3.5 Sonnet)
OPENROUTER_API_KEY=sk-or-v1-...

# Firecrawl — scrape + crawl
FIRECRAWL_API_KEY=fc-...

# Exa — search
EXA_API_KEY=...

# Supabase
SUPABASE_URL=https://xxx.supabase.co
SUPABASE_ANON_KEY=eyJ...
SUPABASE_SERVICE_ROLE_KEY=eyJ...

# Stripe
STRIPE_SECRET_KEY=sk_live_...
STRIPE_WEBHOOK_SECRET=whsec_...
STRIPE_PRO_PRICE_ID=price_...       # $9/mo recurring
STRIPE_TOPUP_PRICE_ID=price_...     # $5 one-time, 30 credits

# App
NEXT_PUBLIC_APP_URL=https://api.shepherd.codes
```

## Database Schema

```sql
-- Extends Supabase auth.users
create table public.profiles (
  id                     uuid primary key references auth.users(id),
  email                  text not null,
  github_handle          text,
  plan                   text not null default 'free',  -- 'free' | 'pro'
  stripe_customer_id     text,
  stripe_subscription_id text,
  credits_balance        integer not null default 0,
  created_at             timestamptz not null default now(),
  updated_at             timestamptz not null default now()
);

-- Append-only credit ledger (never update, only insert)
create table public.credit_transactions (
  id            uuid primary key default gen_random_uuid(),
  user_id       uuid not null references public.profiles(id),
  amount        integer not null,                         -- positive = grant, negative = spend
  balance_after integer not null,
  type          text not null,                            -- subscription_grant | topup | generation | refund
  description   text,
  generation_id uuid,
  created_at    timestamptz not null default now()
);

-- Generation history (all 8 features)
create table public.generations (
  id           uuid primary key default gen_random_uuid(),
  user_id      uuid not null references public.profiles(id),
  type         text not null,                             -- logo | name | northstar | scrape | crawl | vision | search
  credits_used integer not null,
  input_params jsonb,
  result       jsonb,
  status       text not null default 'pending',           -- pending | complete | failed
  created_at   timestamptz not null default now()
);

-- Trial tracking — 2 free uses per feature, per user
create table public.trial_usage (
  id             uuid primary key default gen_random_uuid(),
  user_id        uuid not null references public.profiles(id),
  feature        text not null,
  uses_remaining integer not null default 2,
  unique (user_id, feature)
);

-- Async crawl jobs
create table public.crawl_jobs (
  id                  uuid primary key default gen_random_uuid(),
  user_id             uuid not null references public.profiles(id),
  firecrawl_crawl_id  text not null,
  status              text not null default 'pending',
  generation_id       uuid references public.generations(id),
  created_at          timestamptz not null default now()
);

-- Atomic credit check-and-deduct (prevents race conditions)
create function deduct_credits(
  p_user_id    uuid,
  p_amount     int,
  p_description text
) returns int language plpgsql as $$
declare
  v_balance int;
begin
  select credits_balance into v_balance
  from public.profiles
  where id = p_user_id
  for update;                    -- row lock

  if v_balance < p_amount then
    raise exception 'insufficient_credits: need %, have %', p_amount, v_balance;
  end if;

  update public.profiles
  set credits_balance = credits_balance - p_amount,
      updated_at = now()
  where id = p_user_id
  returning credits_balance into v_balance;

  insert into public.credit_transactions
    (user_id, amount, balance_after, type, description)
  values
    (p_user_id, -p_amount, v_balance, 'generation', p_description);

  return v_balance;
end;
$$;

-- Row Level Security
alter table public.profiles           enable row level security;
alter table public.credit_transactions enable row level security;
alter table public.generations        enable row level security;
alter table public.trial_usage        enable row level security;
alter table public.crawl_jobs         enable row level security;

-- Users can only read their own rows; all writes via service-role
create policy "own rows" on public.profiles           for select using (auth.uid() = id);
create policy "own rows" on public.credit_transactions for select using (auth.uid() = user_id);
create policy "own rows" on public.generations        for select using (auth.uid() = user_id);
create policy "own rows" on public.trial_usage        for select using (auth.uid() = user_id);
create policy "own rows" on public.crawl_jobs         for select using (auth.uid() = user_id);
```

## Auth Flow

```
1. shep opens browser → GET /api/auth/login?provider=github (or ?email=...)
2. Supabase handles OAuth / sends magic link
3. Callback: Supabase → /api/auth/callback → redirect to
   shepherd://auth/callback?access_token=JWT&refresh_token=...
4. Tauri catches deep link → store JWT at ~/.shepherd/.jwt (mode 0600)
5. App polls GET /api/credits/balance → populate ~/.shepherd/auth.toml cache
```

## Stripe Lifecycle

| Event | Handler action |
|-------|---------------|
| `checkout.session.completed` (sub) | set plan=pro, upsert stripe IDs |
| `invoice.payment_succeeded` (monthly) | reset credits_balance=50, log transaction |
| `checkout.session.completed` (topup) | +30 credits, log transaction |
| `customer.subscription.deleted` | plan=free, credits_balance=0 |

Webhook handler verifies Stripe signature before any DB writes.

## Generation Routes

All 8 generation routes share the same middleware pipeline:

```
verifyJwt → checkTrialOrDeductCredits → callUpstream → saveGeneration → respond
```

### Upstream Models & Services

| Route | Upstream | Model / API |
|-------|----------|-------------|
| `POST /api/generate/logo` | OpenRouter | `ideogram/ideogram-v2` — 4 variants |
| `POST /api/generate/name` | OpenRouter + RDAP | `anthropic/claude-3.5-haiku` + domain checks |
| `POST /api/generate/northstar` | OpenRouter | `anthropic/claude-3.5-sonnet` |
| `POST /api/generate/scrape` | Firecrawl | `/scrape` → markdown |
| `POST /api/generate/crawl` | Firecrawl | `/crawl` async → crawl_id |
| `GET  /api/generate/crawl/[id]` | Firecrawl | `/crawl/{id}` poll |
| `POST /api/generate/vision` | OpenRouter | `anthropic/claude-3.5-sonnet` (vision) |
| `POST /api/generate/search` | Exa | `/search` neural + keyword |

### Credit Costs

| Feature | Credits | Trial uses |
|---------|---------|-----------|
| logo | 2 | 2 |
| name | 1 | 2 |
| northstar | 15 | 2 |
| scrape | 1 | 2 |
| crawl | 5 | 2 |
| vision | 2 | 2 |
| search | 1 | 2 |

### Error Response Contract

```json
401  { "error": "auth_required" }
402  { "error": "credits_insufficient", "need": 2, "have": 0, "upgrade_url": "..." }
402  { "error": "trial_exhausted", "upgrade_url": "..." }
429  { "error": "rate_limited", "retry_after": 60 }
5xx  { "error": "upstream_error", "message": "..." }
```

### Balance Response Contract

```json
{
  "plan": "pro",
  "credits_balance": 42,
  "email": "user@example.com",
  "github_handle": "h4x0r",
  "trial_logo": 2,
  "trial_name": 0,
  "trial_northstar": 1,
  "trial_scrape": 2,
  "trial_crawl": 2,
  "trial_vision": 2,
  "trial_search": 2
}
```

## TDD Strategy

Framework: **Vitest** with **MSW** (Mock Service Worker) intercepting all upstream calls.
No real API keys in tests. Each route gets a test file with 4 cases minimum:
`200 ok | 401 unauthed | 402 no credits | 5xx upstream error`.

Stripe webhook tests use `stripe.webhooks.constructEvent` with a test secret.

Run: `npx vitest run` for CI, `npx vitest` for watch mode.

## Security

- JWT verified on every request via `supabase.auth.getUser(token)`
- RLS at DB level — defense in depth even if route has a bug
- Stripe webhook signature verified before any state mutation
- Service-role key never sent to client
- CORS: only `shepherd://` scheme and `*.shepherd.codes`
- Rate limit: 10 req/min per user per endpoint (Upstash Ratelimit)
- All API keys (OpenRouter, Firecrawl, Exa) server-side only, never in responses

## Desktop App Change

Update `DEFAULT_API_URL` in `crates/shepherd-core/src/cloud/mod.rs`:
```rust
// from:
pub const DEFAULT_API_URL: &str = "https://shepherd-pro.vercel.app";
// to:
pub const DEFAULT_API_URL: &str = "https://api.shepherd.codes";
```
