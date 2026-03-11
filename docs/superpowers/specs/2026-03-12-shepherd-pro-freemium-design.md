# Shepherd Pro — Freemium Cloud Backend Design

## Overview

Shepherd is a free desktop app for managing AI coding agents. Shepherd Pro adds a cloud backend that unlocks generative features (logo gen, name gen, North Star wizard) and removes agent/task caps via a $9/mo subscription with a credits system.

**Core principle:** The desktop app works fully without login. Cloud features activate at natural friction points — hitting the agent cap or clicking a generative feature.

## Pricing Model

### Tiers

| | Free (no login) | Free (logged in) | Pro ($9/mo) |
|---|---|---|---|
| Concurrent agents | 3 | 3 | Unlimited |
| Active tasks | 10 | 10 | Unlimited |
| Name generator | — | 2 trials | 1 credit/use |
| Logo generator | — | 2 trials | 2 credits/use |
| North Star wizard | — | 2 trials | 15 credits/use |
| Credits | — | — | 50/mo |
| Credit top-ups | — | — | $5/30 credits |
| Usage analytics | — | Basic | Full history |

### Credit Economics

| Feature | API cost per use | Credits charged | Margin |
|---------|-----------------|----------------|--------|
| Name gen (20 candidates + WHOIS) | ~$0.08 | 1 ($0.18) | +$0.10 |
| Logo gen (4 Ideogram variants) | ~$0.16 | 2 ($0.36) | +$0.20 |
| North Star (13-phase LLM) | ~$2.00 | 15 ($2.70) | +$0.70 |

Credit value: $0.18/credit (subscription), $0.17/credit (top-up).

### Credit Rules

- **Monthly credits don't roll over.** Creates usage urgency, keeps costs predictable.
- **Top-up credits do roll over.** User paid separately for them.
- **Cancellation zeroes credit balance.** Past generations remain accessible.
- **Credit pool is shared** across all generative features. User decides allocation.

### Trial System

- Each generative feature gets **2 free uses** for logged-in users.
- Trials don't consume credits — tracked separately in `trial_usage` table.
- Purpose: let users experience the value before paying.
- Trials require login to capture identity for funnel tracking.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  SHEPHERD DESKTOP (Tauri 2.0)                            │
│  ┌─────────────────────────────────────────────────────┐ │
│  │ Existing Rust Backend (SQLite, PTY, Adapters, etc.) │ │
│  │                                                     │ │
│  │ NEW: Cloud Client Module                            │ │
│  │  ├── AuthClient (Supabase SDK)                      │ │
│  │  ├── CreditClient (check balance, deduct)           │ │
│  │  └── GenerationClient (logo, name, northstar)       │ │
│  └─────────────────────────────────────────────────────┘ │
│           │ HTTPS (JWT in Authorization header)          │
└───────────┼──────────────────────────────────────────────┘
            ▼
┌──────────────────────────────────────────────────────────┐
│  VERCEL — api.shepherd.codes (Next.js API Routes)        │
│  ├── /api/auth/login          Initiate OAuth/magic link  │
│  ├── /api/auth/callback       Handle callback, issue JWT │
│  ├── /api/credits/balance     GET plan + credits + trials│
│  ├── /api/credits/purchase    POST Stripe checkout       │
│  ├── /api/generate/logo       POST deduct + OpenRouter   │
│  ├── /api/generate/name       POST deduct + OpenRouter   │
│  ├── /api/generate/northstar  POST deduct + OpenRouter   │
│  ├── /api/webhooks/stripe     Stripe payment webhooks    │
│  └── /api/trial/status        GET trial usage counts     │
│           │                                               │
│           ├── OpenRouter API (server-side key)            │
│           └── Stripe API                                  │
└───────────┼──────────────────────────────────────────────┘
            ▼
┌──────────────────────────────────────────────────────────┐
│  SUPABASE                                                │
│  ├── Auth (GitHub OAuth + Magic Link)                    │
│  ├── profiles (plan, credits_balance, stripe IDs)        │
│  ├── credit_transactions (append-only ledger)            │
│  ├── generations (type, prompt, result, credits_used)    │
│  ├── trial_usage (user_id, feature, uses_remaining)      │
│  └── Row Level Security on all tables                    │
└──────────────────────────────────────────────────────────┘
```

### Why This Architecture

- **Server-side generation:** API key never touches the client. Credit deduction is server-authoritative — can't be spoofed.
- **Supabase for auth + DB:** Handles OAuth, magic links, JWT verification, Postgres with RLS out of the box. Generous free tier.
- **Vercel for API routes:** Serverless, edge-cached, auto-scaling. Next.js API routes are the industry standard for this pattern.
- **Separate repos:** Desktop app (SecurityRonin/Shepherd) and cloud backend (SecurityRonin/shepherd-pro) are independent. Contract is API routes + JWT format.

## Database Schema

```sql
-- Extends Supabase auth.users
create table public.profiles (
  id                     uuid primary key references auth.users(id),
  email                  text not null,
  github_handle          text,
  plan                   text not null default 'free',
  stripe_customer_id     text,
  stripe_subscription_id text,
  credits_balance        integer not null default 0,
  created_at             timestamptz not null default now(),
  updated_at             timestamptz not null default now()
);

-- Append-only credit ledger
create table public.credit_transactions (
  id            uuid primary key default gen_random_uuid(),
  user_id       uuid not null references public.profiles(id),
  amount        integer not null,
  balance_after integer not null,
  type          text not null,  -- 'subscription_grant' | 'topup' | 'generation' | 'refund'
  description   text,
  generation_id uuid,
  created_at    timestamptz not null default now()
);

-- Generation history
create table public.generations (
  id            uuid primary key default gen_random_uuid(),
  user_id       uuid not null references public.profiles(id),
  type          text not null,  -- 'logo' | 'name' | 'northstar'
  credits_used  integer not null,
  input_prompt  text,
  input_params  jsonb,
  result        jsonb,
  status        text not null default 'pending',
  created_at    timestamptz not null default now()
);

-- Trial tracking
create table public.trial_usage (
  id             uuid primary key default gen_random_uuid(),
  user_id        uuid not null references public.profiles(id),
  feature        text not null,
  uses_remaining integer not null default 2,
  unique (user_id, feature)
);

-- Row Level Security on all tables
-- Users can only read their own data
-- Writes go through API routes only (service role key)
```

## Stripe Integration

### Subscription Lifecycle

1. User clicks "Upgrade to Pro" in desktop app
2. App opens system browser to `/api/credits/purchase`
3. Route creates Stripe Checkout Session ($9/mo recurring)
4. User completes payment on Stripe's hosted page
5. `invoice.payment_succeeded` webhook fires
6. Handler: set plan=pro, grant 50 credits, log transaction
7. Desktop app polls `/api/credits/balance`, sees Pro status

### Monthly Renewal

- Stripe fires `invoice.payment_succeeded` on each billing cycle
- Handler resets `credits_balance` to 50 (unused credits expire)
- Inserts `credit_transaction` (+50, type: subscription_grant)

### Top-Up Purchase

- Stripe Checkout (one-time payment, $5)
- On success: insert +30 credits (type: topup)
- Top-up credits are additive and roll over

### Cancellation

- `customer.subscription.deleted` webhook
- Set plan=free, credits_balance=0
- Past generations remain accessible

## Desktop App Integration

### Auth Flow

1. User clicks "Sign in" (prompted by agent cap or generative feature)
2. Tauri opens system browser to `https://api.shepherd.codes/api/auth/login`
3. Supabase handles GitHub OAuth or magic link
4. Callback redirects to `shepherd://auth/callback?token=<jwt>`
5. Tauri catches deep link via custom URL scheme registration
6. JWT stored in OS keychain (macOS Keychain, libsecret, Windows Credential Manager)

### Local State

```
~/.shepherd/
├── auth.toml    # NEW — cached profile (non-sensitive)
│   ├── user_id
│   ├── email
│   ├── plan
│   ├── credits_balance   # refreshed on app start + after mutations
│   └── trial_counts      # { logo: 2, name: 1, northstar: 2 }
```

- JWT in OS keychain — never on disk in plaintext
- `auth.toml` is a display cache — server is authoritative
- Agent/task caps enforced locally from cached plan field
- Generative features always verified server-side

### Offline Behavior

| Scenario | Behavior |
|----------|----------|
| No login, <3 agents | Full orchestration, no prompts |
| No login, 4th agent | Soft prompt: "Sign in to unlock unlimited agents" |
| Logged in free, trial available | Trial works (requires internet) |
| Logged in Pro, has credits | Full functionality |
| Logged in Pro, no internet | Orchestration works. Generators show "Offline" |

### New Rust Modules

```
crates/shepherd-core/src/cloud/
├── mod.rs          # module registry
├── auth.rs         # Supabase auth, JWT management, keychain storage
├── credits.rs      # Credit balance check, deduction requests
├── generation.rs   # Logo/name/northstar API calls via Vercel
└── sync.rs         # Background refresh of balance and plan status
```

## Vercel Project Structure

```
shepherd-pro/                       # SecurityRonin/shepherd-pro
├── .env.local                      # Secrets (gitignored)
├── .env.example
├── next.config.ts
├── package.json
├── tsconfig.json
├── supabase/
│   ├── migrations/
│   │   └── 001_initial_schema.sql
│   └── config.toml
├── src/
│   ├── lib/
│   │   ├── supabase-admin.ts       # Service role client (writes)
│   │   ├── supabase-auth.ts        # Anon client (JWT verification)
│   │   ├── stripe.ts               # Stripe client + helpers
│   │   ├── openrouter.ts           # OpenRouter API client
│   │   ├── credits.ts              # Atomic credit check + deduct
│   │   └── middleware.ts           # Auth middleware
│   ├── app/
│   │   └── api/
│   │       ├── auth/
│   │       │   ├── login/route.ts
│   │       │   └── callback/route.ts
│   │       ├── credits/
│   │       │   ├── balance/route.ts
│   │       │   └── purchase/route.ts
│   │       ├── generate/
│   │       │   ├── logo/route.ts
│   │       │   ├── name/route.ts
│   │       │   └── northstar/route.ts
│   │       └── webhooks/
│   │           └── stripe/route.ts
│   └── types/
│       └── index.ts
└── tests/
    ├── credits.test.ts
    ├── generate.test.ts
    └── webhooks.test.ts
```

### Environment Variables

```bash
OPENROUTER_API_KEY=sk-or-v1-...
SUPABASE_URL=https://xxx.supabase.co
SUPABASE_ANON_KEY=eyJ...
SUPABASE_SERVICE_ROLE_KEY=eyJ...
STRIPE_SECRET_KEY=sk_live_...
STRIPE_WEBHOOK_SECRET=whsec_...
STRIPE_PRO_PRICE_ID=price_...
STRIPE_TOPUP_PRICE_ID=price_...
NEXT_PUBLIC_APP_URL=https://api.shepherd.codes
```

### Deployment

- **Production:** `api.shepherd.codes` (Vercel custom domain)
- **Preview:** Auto-deploy on PR
- **Dev:** `localhost:3000`

## Generation API Contracts

### POST /api/generate/logo

```json
// Request
{
  "style": "minimal",         // minimal | geometric | mascot | abstract
  "colors": ["#000000"],      // optional
  "description": "Developer tool for managing AI agents"
}

// Response (200)
{
  "generation_id": "uuid",
  "images": [
    { "url": "https://...", "variant": 1 },
    { "url": "https://...", "variant": 2 },
    { "url": "https://...", "variant": 3 },
    { "url": "https://...", "variant": 4 }
  ],
  "credits_used": 2,
  "credits_remaining": 48
}
```

### POST /api/generate/name

```json
// Request
{
  "description": "AI productivity tool",
  "vibes": ["bold", "minimal"],      // optional
  "check_domains": true              // WHOIS + npm + PyPI + GitHub
}

// Response (200)
{
  "generation_id": "uuid",
  "candidates": [
    {
      "name": "Shepherd",
      "domain_available": true,
      "npm_available": true,
      "pypi_available": false,
      "github_available": true
    }
  ],
  "credits_used": 1,
  "credits_remaining": 47
}
```

### POST /api/generate/northstar

```json
// Request
{
  "phase": 1,                        // 1-13, sequential
  "inputs": { ... }                  // Phase-specific inputs
}

// Response (200)
{
  "generation_id": "uuid",
  "phase": 1,
  "document": "# Brand Guidelines\n...",
  "credits_used": 15,
  "credits_remaining": 32,
  "next_phase": 2
}
```

### Error Responses

```json
// 401 — Not authenticated
{ "error": "auth_required", "message": "Sign in to use this feature" }

// 402 — Payment required
{ "error": "credits_insufficient", "message": "Need 2 credits, have 0", "upgrade_url": "..." }

// 402 — Trial exhausted
{ "error": "trial_exhausted", "message": "Free trials used. Upgrade to Pro.", "upgrade_url": "..." }

// 429 — Rate limited
{ "error": "rate_limited", "message": "Too many requests. Try again in 60s." }
```

## Security

- **API key server-side only** — OpenRouter key never sent to client
- **JWT verification on every request** — Supabase verifies signature
- **RLS at database level** — Defense in depth, even buggy API routes can't leak cross-user data
- **Stripe webhook signature verification** — Prevents spoofed payment events
- **OS keychain for JWT storage** — Not written to disk in plaintext
- **Rate limiting** — Per-user, per-endpoint rate limits to prevent abuse
- **CORS** — API routes only accept requests from `shepherd://` scheme and `shepherd.codes`
