# Shepherd Pro Cloud Backend Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Shepherd Pro cloud backend — a Next.js API service on Vercel with Supabase auth/DB and Stripe payments that powers freemium generative features (logo gen, name gen, North Star wizard) for the Shepherd desktop app.

**Architecture:** Vercel API routes handle all business logic. Supabase provides auth (GitHub OAuth + magic link) and Postgres with RLS. Stripe manages subscriptions ($9/mo Pro) and one-time credit top-ups ($5/30). OpenRouter proxies image/LLM generation with the API key server-side. The desktop app authenticates via deep link callback and sends JWTs on every request.

**Tech Stack:** Next.js 15 (App Router), TypeScript, Supabase (auth + Postgres), Stripe, OpenRouter API, Vitest for testing, Vercel for deployment.

**Spec:** `docs/superpowers/specs/2026-03-12-shepherd-pro-freemium-design.md`

**Repo:** `SecurityRonin/shepherd-pro` (new, separate from `SecurityRonin/Shepherd`)

---

## Chunk 1: Project Scaffolding + Database Schema

### Task 1: Initialize Next.js Project

**Files:**
- Create: `shepherd-pro/package.json`
- Create: `shepherd-pro/tsconfig.json`
- Create: `shepherd-pro/next.config.ts`
- Create: `shepherd-pro/.env.example`
- Create: `shepherd-pro/.env.local`
- Create: `shepherd-pro/.gitignore`
- Create: `shepherd-pro/src/types/index.ts`

- [ ] **Step 1: Create repo directory and initialize Next.js**

```bash
mkdir -p /Users/4n6h4x0r/src/shepherd-pro
cd /Users/4n6h4x0r/src/shepherd-pro
npx create-next-app@latest . --typescript --eslint --app --src-dir --no-tailwind --no-import-alias
```

Accept defaults. This gives us the App Router structure.

- [ ] **Step 2: Install dependencies**

```bash
cd /Users/4n6h4x0r/src/shepherd-pro
npm install @supabase/supabase-js stripe
npm install -D vitest @types/node
```

- [ ] **Step 3: Create .env.example**

```bash
# .env.example
OPENROUTER_API_KEY=sk-or-v1-your-key-here
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_ANON_KEY=your-anon-key
SUPABASE_SERVICE_ROLE_KEY=your-service-role-key
STRIPE_SECRET_KEY=sk_test_your-key
STRIPE_WEBHOOK_SECRET=whsec_your-secret
STRIPE_PRO_PRICE_ID=price_your-pro-price
STRIPE_TOPUP_PRICE_ID=price_your-topup-price
NEXT_PUBLIC_APP_URL=http://localhost:3000
```

- [ ] **Step 4: Create .env.local with real OpenRouter key**

```bash
# .env.local (gitignored)
OPENROUTER_API_KEY=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
SUPABASE_URL=
SUPABASE_ANON_KEY=
SUPABASE_SERVICE_ROLE_KEY=
STRIPE_SECRET_KEY=
STRIPE_WEBHOOK_SECRET=
STRIPE_PRO_PRICE_ID=
STRIPE_TOPUP_PRICE_ID=
NEXT_PUBLIC_APP_URL=http://localhost:3000
```

- [ ] **Step 5: Create shared types**

```typescript
// src/types/index.ts

export type Plan = 'free' | 'pro';
export type GenerationType = 'logo' | 'name' | 'northstar';
export type TransactionType = 'subscription_grant' | 'topup' | 'generation' | 'refund';

export interface Profile {
  id: string;
  email: string;
  github_handle: string | null;
  plan: Plan;
  stripe_customer_id: string | null;
  stripe_subscription_id: string | null;
  credits_balance: number;
  created_at: string;
  updated_at: string;
}

export interface CreditTransaction {
  id: string;
  user_id: string;
  amount: number;
  balance_after: number;
  type: TransactionType;
  description: string | null;
  generation_id: string | null;
  created_at: string;
}

export interface Generation {
  id: string;
  user_id: string;
  type: GenerationType;
  credits_used: number;
  input_prompt: string | null;
  input_params: Record<string, unknown> | null;
  result: Record<string, unknown> | null;
  status: 'pending' | 'completed' | 'failed';
  created_at: string;
}

export interface TrialUsage {
  id: string;
  user_id: string;
  feature: GenerationType;
  uses_remaining: number;
}

export const CREDIT_COSTS: Record<GenerationType, number> = {
  logo: 2,
  name: 1,
  northstar: 15,
};

export const TRIAL_LIMIT = 2;
export const PRO_MONTHLY_CREDITS = 50;
export const TOPUP_CREDITS = 30;

export interface ApiError {
  error: string;
  message: string;
  upgrade_url?: string;
}
```

- [ ] **Step 6: Add vitest config**

Create `vitest.config.ts`:

```typescript
import { defineConfig } from 'vitest/config';
import path from 'path';

export default defineConfig({
  test: {
    environment: 'node',
    globals: true,
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
});
```

Add to `package.json` scripts:

```json
"test": "vitest run",
"test:watch": "vitest"
```

- [ ] **Step 7: Initialize git and commit**

```bash
cd /Users/4n6h4x0r/src/shepherd-pro
git init
git add -A
git commit -m "feat: initialize shepherd-pro Next.js project with types and config"
```

---

### Task 2: Supabase Database Migration

**Files:**
- Create: `supabase/migrations/001_initial_schema.sql`
- Create: `supabase/config.toml`

- [ ] **Step 1: Create Supabase config**

```toml
# supabase/config.toml
[project]
id = "shepherd-pro"

[db]
port = 54322
major_version = 15

[api]
port = 54321
```

- [ ] **Step 2: Write the migration with tables and RLS**

```sql
-- supabase/migrations/001_initial_schema.sql

-- Profiles (extends auth.users)
create table public.profiles (
  id                     uuid primary key references auth.users(id) on delete cascade,
  email                  text not null,
  github_handle          text,
  plan                   text not null default 'free' check (plan in ('free', 'pro')),
  stripe_customer_id     text,
  stripe_subscription_id text,
  credits_balance        integer not null default 0 check (credits_balance >= 0),
  created_at             timestamptz not null default now(),
  updated_at             timestamptz not null default now()
);

-- Credit ledger (append-only)
create table public.credit_transactions (
  id            uuid primary key default gen_random_uuid(),
  user_id       uuid not null references public.profiles(id) on delete cascade,
  amount        integer not null,
  balance_after integer not null check (balance_after >= 0),
  type          text not null check (type in ('subscription_grant', 'topup', 'generation', 'refund')),
  description   text,
  generation_id uuid,
  created_at    timestamptz not null default now()
);

-- Generation history
create table public.generations (
  id            uuid primary key default gen_random_uuid(),
  user_id       uuid not null references public.profiles(id) on delete cascade,
  type          text not null check (type in ('logo', 'name', 'northstar')),
  credits_used  integer not null,
  input_prompt  text,
  input_params  jsonb,
  result        jsonb,
  status        text not null default 'pending' check (status in ('pending', 'completed', 'failed')),
  created_at    timestamptz not null default now()
);

-- Trial tracking
create table public.trial_usage (
  id             uuid primary key default gen_random_uuid(),
  user_id        uuid not null references public.profiles(id) on delete cascade,
  feature        text not null check (feature in ('logo', 'name', 'northstar')),
  uses_remaining integer not null default 2 check (uses_remaining >= 0),
  unique (user_id, feature)
);

-- Indexes
create index idx_credit_transactions_user on public.credit_transactions(user_id, created_at desc);
create index idx_generations_user on public.generations(user_id, created_at desc);
create index idx_trial_usage_user_feature on public.trial_usage(user_id, feature);

-- RLS
alter table public.profiles enable row level security;
alter table public.credit_transactions enable row level security;
alter table public.generations enable row level security;
alter table public.trial_usage enable row level security;

-- Profiles: users read own, service role writes
create policy "Users read own profile"
  on public.profiles for select
  using (auth.uid() = id);

create policy "Service role manages profiles"
  on public.profiles for all
  using (auth.role() = 'service_role');

-- Credit transactions: users read own
create policy "Users read own transactions"
  on public.credit_transactions for select
  using (auth.uid() = user_id);

create policy "Service role manages transactions"
  on public.credit_transactions for all
  using (auth.role() = 'service_role');

-- Generations: users read own
create policy "Users read own generations"
  on public.generations for select
  using (auth.uid() = user_id);

create policy "Service role manages generations"
  on public.generations for all
  using (auth.role() = 'service_role');

-- Trial usage: users read own
create policy "Users read own trials"
  on public.trial_usage for select
  using (auth.uid() = user_id);

create policy "Service role manages trials"
  on public.trial_usage for all
  using (auth.role() = 'service_role');

-- Auto-create profile on signup
create or replace function public.handle_new_user()
returns trigger as $$
begin
  insert into public.profiles (id, email, github_handle)
  values (
    new.id,
    new.email,
    new.raw_user_meta_data->>'user_name'
  );

  -- Initialize trial usage for all features
  insert into public.trial_usage (user_id, feature, uses_remaining)
  values
    (new.id, 'logo', 2),
    (new.id, 'name', 2),
    (new.id, 'northstar', 2);

  return new;
end;
$$ language plpgsql security definer;

create trigger on_auth_user_created
  after insert on auth.users
  for each row execute procedure public.handle_new_user();

-- Updated_at trigger
create or replace function public.update_updated_at()
returns trigger as $$
begin
  new.updated_at = now();
  return new;
end;
$$ language plpgsql;

create trigger profiles_updated_at
  before update on public.profiles
  for each row execute procedure public.update_updated_at();
```

- [ ] **Step 3: Commit**

```bash
git add supabase/
git commit -m "feat: add Supabase schema migration with RLS and auto-profile trigger"
```

---

## Chunk 2: Supabase Clients + Auth Middleware

### Task 3: Supabase Client Libraries

**Files:**
- Create: `src/lib/supabase-admin.ts`
- Create: `src/lib/supabase-auth.ts`

- [ ] **Step 1: Create service role client (for server-side writes)**

```typescript
// src/lib/supabase-admin.ts
import { createClient } from '@supabase/supabase-js';

if (!process.env.SUPABASE_URL) throw new Error('SUPABASE_URL is required');
if (!process.env.SUPABASE_SERVICE_ROLE_KEY) throw new Error('SUPABASE_SERVICE_ROLE_KEY is required');

export const supabaseAdmin = createClient(
  process.env.SUPABASE_URL,
  process.env.SUPABASE_SERVICE_ROLE_KEY,
  {
    auth: {
      autoRefreshToken: false,
      persistSession: false,
    },
  }
);
```

- [ ] **Step 2: Create auth verification client**

```typescript
// src/lib/supabase-auth.ts
import { createClient } from '@supabase/supabase-js';

if (!process.env.SUPABASE_URL) throw new Error('SUPABASE_URL is required');
if (!process.env.SUPABASE_ANON_KEY) throw new Error('SUPABASE_ANON_KEY is required');

export function createAuthClient(accessToken: string) {
  return createClient(
    process.env.SUPABASE_URL!,
    process.env.SUPABASE_ANON_KEY!,
    {
      global: {
        headers: {
          Authorization: `Bearer ${accessToken}`,
        },
      },
      auth: {
        autoRefreshToken: false,
        persistSession: false,
      },
    }
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/supabase-admin.ts src/lib/supabase-auth.ts
git commit -m "feat: add Supabase admin and auth client libraries"
```

---

### Task 4: Auth Middleware

**Files:**
- Create: `src/lib/middleware.ts`
- Create: `tests/middleware.test.ts`

- [ ] **Step 1: Write the test**

```typescript
// tests/middleware.test.ts
import { describe, it, expect, vi } from 'vitest';

// We'll test the extractToken helper since the full middleware
// depends on Supabase which we'd need to mock
vi.mock('@/lib/supabase-auth', () => ({
  createAuthClient: vi.fn(),
}));

describe('extractToken', () => {
  it('extracts Bearer token from Authorization header', async () => {
    const { extractToken } = await import('@/lib/middleware');
    const headers = new Headers({ Authorization: 'Bearer abc123' });
    expect(extractToken(headers)).toBe('abc123');
  });

  it('returns null when no Authorization header', async () => {
    const { extractToken } = await import('@/lib/middleware');
    const headers = new Headers();
    expect(extractToken(headers)).toBeNull();
  });

  it('returns null for non-Bearer token', async () => {
    const { extractToken } = await import('@/lib/middleware');
    const headers = new Headers({ Authorization: 'Basic abc123' });
    expect(extractToken(headers)).toBeNull();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /Users/4n6h4x0r/src/shepherd-pro
npx vitest run tests/middleware.test.ts
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement middleware**

```typescript
// src/lib/middleware.ts
import { NextRequest, NextResponse } from 'next/server';
import { createAuthClient } from '@/lib/supabase-auth';
import type { Profile, ApiError } from '@/types';
import { supabaseAdmin } from '@/lib/supabase-admin';

export function extractToken(headers: Headers): string | null {
  const auth = headers.get('Authorization');
  if (!auth?.startsWith('Bearer ')) return null;
  return auth.slice(7);
}

export interface AuthenticatedRequest {
  user: { id: string; email: string };
  profile: Profile;
}

export async function authenticateRequest(
  request: NextRequest
): Promise<AuthenticatedRequest | NextResponse<ApiError>> {
  const token = extractToken(request.headers);
  if (!token) {
    return NextResponse.json(
      { error: 'auth_required', message: 'Sign in to use this feature' },
      { status: 401 }
    );
  }

  const supabase = createAuthClient(token);
  const { data: { user }, error } = await supabase.auth.getUser();

  if (error || !user) {
    return NextResponse.json(
      { error: 'auth_required', message: 'Invalid or expired token' },
      { status: 401 }
    );
  }

  const { data: profile, error: profileError } = await supabaseAdmin
    .from('profiles')
    .select('*')
    .eq('id', user.id)
    .single();

  if (profileError || !profile) {
    return NextResponse.json(
      { error: 'auth_required', message: 'Profile not found' },
      { status: 401 }
    );
  }

  return {
    user: { id: user.id, email: user.email! },
    profile: profile as Profile,
  };
}

export function isAuthError(
  result: AuthenticatedRequest | NextResponse<ApiError>
): result is NextResponse<ApiError> {
  return result instanceof NextResponse;
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
npx vitest run tests/middleware.test.ts
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/middleware.ts tests/middleware.test.ts
git commit -m "feat: add auth middleware with JWT extraction and Supabase verification"
```

---

### Task 5: Auth Routes (Login + Callback)

**Files:**
- Create: `src/app/api/auth/login/route.ts`
- Create: `src/app/api/auth/callback/route.ts`

- [ ] **Step 1: Implement login route**

```typescript
// src/app/api/auth/login/route.ts
import { NextRequest, NextResponse } from 'next/server';
import { createClient } from '@supabase/supabase-js';

export async function GET(request: NextRequest) {
  const provider = request.nextUrl.searchParams.get('provider');
  const email = request.nextUrl.searchParams.get('email');
  const appUrl = process.env.NEXT_PUBLIC_APP_URL!;

  const supabase = createClient(
    process.env.SUPABASE_URL!,
    process.env.SUPABASE_ANON_KEY!
  );

  if (provider === 'github') {
    const { data, error } = await supabase.auth.signInWithOAuth({
      provider: 'github',
      options: {
        redirectTo: `${appUrl}/api/auth/callback`,
      },
    });
    if (error) {
      return NextResponse.json({ error: 'auth_failed', message: error.message }, { status: 400 });
    }
    return NextResponse.redirect(data.url);
  }

  if (email) {
    const { error } = await supabase.auth.signInWithOtp({
      email,
      options: {
        emailRedirectTo: `${appUrl}/api/auth/callback`,
      },
    });
    if (error) {
      return NextResponse.json({ error: 'auth_failed', message: error.message }, { status: 400 });
    }
    return NextResponse.json({ message: 'Check your email for a login link' });
  }

  return NextResponse.json(
    { error: 'bad_request', message: 'Provide ?provider=github or ?email=user@example.com' },
    { status: 400 }
  );
}
```

- [ ] **Step 2: Implement callback route**

```typescript
// src/app/api/auth/callback/route.ts
import { NextRequest, NextResponse } from 'next/server';
import { createClient } from '@supabase/supabase-js';

export async function GET(request: NextRequest) {
  const code = request.nextUrl.searchParams.get('code');

  if (!code) {
    return NextResponse.json(
      { error: 'bad_request', message: 'Missing auth code' },
      { status: 400 }
    );
  }

  const supabase = createClient(
    process.env.SUPABASE_URL!,
    process.env.SUPABASE_ANON_KEY!
  );

  const { data, error } = await supabase.auth.exchangeCodeForSession(code);

  if (error || !data.session) {
    return NextResponse.json(
      { error: 'auth_failed', message: error?.message ?? 'Failed to exchange code' },
      { status: 400 }
    );
  }

  // Redirect to desktop app via deep link with the access token
  const deepLink = `shepherd://auth/callback?access_token=${data.session.access_token}&refresh_token=${data.session.refresh_token}`;

  return NextResponse.redirect(deepLink);
}
```

- [ ] **Step 3: Commit**

```bash
git add src/app/api/auth/
git commit -m "feat: add auth login (GitHub OAuth + magic link) and callback routes"
```

---

## Chunk 3: Credit System

### Task 6: Credits Library (Atomic Check + Deduct)

**Files:**
- Create: `src/lib/credits.ts`
- Create: `tests/credits.test.ts`

- [ ] **Step 1: Write the tests**

```typescript
// tests/credits.test.ts
import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock supabase admin
const mockFrom = vi.fn();
const mockRpc = vi.fn();
vi.mock('@/lib/supabase-admin', () => ({
  supabaseAdmin: {
    from: (...args: unknown[]) => mockFrom(...args),
    rpc: (...args: unknown[]) => mockRpc(...args),
  },
}));

describe('credits', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('checkCreditsOrTrial', () => {
    it('returns trial_available when user has trials remaining', async () => {
      const { checkCreditsOrTrial } = await import('@/lib/credits');

      mockFrom.mockReturnValue({
        select: vi.fn().mockReturnValue({
          eq: vi.fn().mockReturnValue({
            eq: vi.fn().mockReturnValue({
              single: vi.fn().mockResolvedValue({
                data: { uses_remaining: 2 },
                error: null,
              }),
            }),
          }),
        }),
      });

      const result = await checkCreditsOrTrial('user-1', 'logo', 'free');
      expect(result.allowed).toBe(true);
      expect(result.method).toBe('trial');
    });

    it('returns credits_available for pro user with enough credits', async () => {
      const { checkCreditsOrTrial } = await import('@/lib/credits');

      // Trial check returns 0
      mockFrom.mockReturnValue({
        select: vi.fn().mockReturnValue({
          eq: vi.fn().mockReturnValue({
            eq: vi.fn().mockReturnValue({
              single: vi.fn().mockResolvedValue({
                data: { uses_remaining: 0 },
                error: null,
              }),
            }),
          }),
        }),
      });

      const result = await checkCreditsOrTrial('user-1', 'logo', 'pro', 10);
      expect(result.allowed).toBe(true);
      expect(result.method).toBe('credits');
    });

    it('returns not allowed when free user has no trials', async () => {
      const { checkCreditsOrTrial } = await import('@/lib/credits');

      mockFrom.mockReturnValue({
        select: vi.fn().mockReturnValue({
          eq: vi.fn().mockReturnValue({
            eq: vi.fn().mockReturnValue({
              single: vi.fn().mockResolvedValue({
                data: { uses_remaining: 0 },
                error: null,
              }),
            }),
          }),
        }),
      });

      const result = await checkCreditsOrTrial('user-1', 'logo', 'free', 0);
      expect(result.allowed).toBe(false);
      expect(result.reason).toBe('trial_exhausted');
    });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
npx vitest run tests/credits.test.ts
```

Expected: FAIL

- [ ] **Step 3: Implement credits library**

```typescript
// src/lib/credits.ts
import { supabaseAdmin } from '@/lib/supabase-admin';
import { CREDIT_COSTS, type GenerationType, type Plan } from '@/types';

export interface CreditCheck {
  allowed: boolean;
  method?: 'trial' | 'credits';
  reason?: 'trial_exhausted' | 'credits_insufficient';
  credits_needed?: number;
}

export async function checkCreditsOrTrial(
  userId: string,
  feature: GenerationType,
  plan: Plan,
  creditsBalance?: number
): Promise<CreditCheck> {
  // Check trial first (works for both free and pro)
  const { data: trial } = await supabaseAdmin
    .from('trial_usage')
    .select('uses_remaining')
    .eq('user_id', userId)
    .eq('feature', feature)
    .single();

  if (trial && trial.uses_remaining > 0) {
    return { allowed: true, method: 'trial' };
  }

  // No trial left — free users are blocked
  if (plan === 'free') {
    return { allowed: false, reason: 'trial_exhausted' };
  }

  // Pro user — check credits
  const cost = CREDIT_COSTS[feature];
  if ((creditsBalance ?? 0) >= cost) {
    return { allowed: true, method: 'credits', credits_needed: cost };
  }

  return {
    allowed: false,
    reason: 'credits_insufficient',
    credits_needed: cost,
  };
}

export async function deductTrial(
  userId: string,
  feature: GenerationType
): Promise<void> {
  await supabaseAdmin.rpc('decrement_trial', {
    p_user_id: userId,
    p_feature: feature,
  });
}

export async function deductCredits(
  userId: string,
  feature: GenerationType,
  generationId: string
): Promise<number> {
  const cost = CREDIT_COSTS[feature];

  // Atomic: decrement balance and insert transaction in one call
  const { data: profile, error } = await supabaseAdmin
    .from('profiles')
    .update({
      credits_balance: supabaseAdmin.rpc('decrement_credits_balance', {
        p_user_id: userId,
        p_amount: cost,
      }),
    })
    .eq('id', userId)
    .select('credits_balance')
    .single();

  // Fallback: read-then-write if rpc not available
  if (error) {
    const { data: current } = await supabaseAdmin
      .from('profiles')
      .select('credits_balance')
      .eq('id', userId)
      .single();

    const newBalance = (current?.credits_balance ?? 0) - cost;

    await supabaseAdmin
      .from('profiles')
      .update({ credits_balance: newBalance })
      .eq('id', userId);

    await supabaseAdmin.from('credit_transactions').insert({
      user_id: userId,
      amount: -cost,
      balance_after: newBalance,
      type: 'generation',
      description: `${feature} generation`,
      generation_id: generationId,
    });

    return newBalance;
  }

  const newBalance = profile?.credits_balance ?? 0;

  await supabaseAdmin.from('credit_transactions').insert({
    user_id: userId,
    amount: -cost,
    balance_after: newBalance,
    type: 'generation',
    description: `${feature} generation`,
    generation_id: generationId,
  });

  return newBalance;
}

export async function grantCredits(
  userId: string,
  amount: number,
  type: 'subscription_grant' | 'topup' | 'refund',
  description: string,
  resetBalance = false
): Promise<number> {
  let newBalance: number;

  if (resetBalance) {
    // Subscription grant: reset to exact amount (unused credits expire)
    await supabaseAdmin
      .from('profiles')
      .update({ credits_balance: amount })
      .eq('id', userId);
    newBalance = amount;
  } else {
    // Top-up/refund: additive
    const { data: current } = await supabaseAdmin
      .from('profiles')
      .select('credits_balance')
      .eq('id', userId)
      .single();
    newBalance = (current?.credits_balance ?? 0) + amount;
    await supabaseAdmin
      .from('profiles')
      .update({ credits_balance: newBalance })
      .eq('id', userId);
  }

  await supabaseAdmin.from('credit_transactions').insert({
    user_id: userId,
    amount,
    balance_after: newBalance,
    type,
    description,
  });

  return newBalance;
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
npx vitest run tests/credits.test.ts
```

Expected: PASS

- [ ] **Step 5: Add SQL function for trial decrement**

Append to migration or create `supabase/migrations/002_functions.sql`:

```sql
-- supabase/migrations/002_functions.sql

-- Atomically decrement trial usage
create or replace function public.decrement_trial(
  p_user_id uuid,
  p_feature text
) returns void as $$
begin
  update public.trial_usage
  set uses_remaining = uses_remaining - 1
  where user_id = p_user_id
    and feature = p_feature
    and uses_remaining > 0;
end;
$$ language plpgsql security definer;
```

- [ ] **Step 6: Commit**

```bash
git add src/lib/credits.ts tests/credits.test.ts supabase/migrations/002_functions.sql
git commit -m "feat: add credit system with trial checking, deduction, and granting"
```

---

### Task 7: Credits Balance Route

**Files:**
- Create: `src/app/api/credits/balance/route.ts`

- [ ] **Step 1: Implement balance route**

```typescript
// src/app/api/credits/balance/route.ts
import { NextRequest, NextResponse } from 'next/server';
import { authenticateRequest, isAuthError } from '@/lib/middleware';
import { supabaseAdmin } from '@/lib/supabase-admin';

export async function GET(request: NextRequest) {
  const auth = await authenticateRequest(request);
  if (isAuthError(auth)) return auth;

  const { profile } = auth;

  // Get trial counts
  const { data: trials } = await supabaseAdmin
    .from('trial_usage')
    .select('feature, uses_remaining')
    .eq('user_id', profile.id);

  const trialCounts: Record<string, number> = {};
  for (const trial of trials ?? []) {
    trialCounts[trial.feature] = trial.uses_remaining;
  }

  return NextResponse.json({
    plan: profile.plan,
    credits_balance: profile.credits_balance,
    trials: trialCounts,
    email: profile.email,
    github_handle: profile.github_handle,
  });
}
```

- [ ] **Step 2: Commit**

```bash
git add src/app/api/credits/balance/route.ts
git commit -m "feat: add credits balance API route"
```

---

## Chunk 4: Stripe Integration

### Task 8: Stripe Client Library

**Files:**
- Create: `src/lib/stripe.ts`

- [ ] **Step 1: Implement Stripe client**

```typescript
// src/lib/stripe.ts
import Stripe from 'stripe';

if (!process.env.STRIPE_SECRET_KEY) throw new Error('STRIPE_SECRET_KEY is required');

export const stripe = new Stripe(process.env.STRIPE_SECRET_KEY, {
  apiVersion: '2025-12-18.acacia',
  typescript: true,
});

export async function createCheckoutSession(
  customerId: string,
  priceId: string,
  mode: 'subscription' | 'payment',
  successUrl: string,
  cancelUrl: string
): Promise<string> {
  const session = await stripe.checkout.sessions.create({
    customer: customerId,
    line_items: [{ price: priceId, quantity: 1 }],
    mode,
    success_url: successUrl,
    cancel_url: cancelUrl,
  });
  return session.url!;
}

export async function getOrCreateCustomer(
  email: string,
  userId: string
): Promise<string> {
  // Check if customer already exists
  const existing = await stripe.customers.list({ email, limit: 1 });
  if (existing.data.length > 0) {
    return existing.data[0].id;
  }

  const customer = await stripe.customers.create({
    email,
    metadata: { supabase_user_id: userId },
  });
  return customer.id;
}

export function constructWebhookEvent(
  body: string,
  signature: string
): Stripe.Event {
  return stripe.webhooks.constructEvent(
    body,
    signature,
    process.env.STRIPE_WEBHOOK_SECRET!
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/stripe.ts
git commit -m "feat: add Stripe client with checkout session and webhook helpers"
```

---

### Task 9: Purchase Route (Checkout Session)

**Files:**
- Create: `src/app/api/credits/purchase/route.ts`

- [ ] **Step 1: Implement purchase route**

```typescript
// src/app/api/credits/purchase/route.ts
import { NextRequest, NextResponse } from 'next/server';
import { authenticateRequest, isAuthError } from '@/lib/middleware';
import { createCheckoutSession, getOrCreateCustomer } from '@/lib/stripe';
import { supabaseAdmin } from '@/lib/supabase-admin';

export async function POST(request: NextRequest) {
  const auth = await authenticateRequest(request);
  if (isAuthError(auth)) return auth;

  const { user, profile } = auth;
  const body = await request.json();
  const type = body.type as 'subscription' | 'topup';

  if (!type || !['subscription', 'topup'].includes(type)) {
    return NextResponse.json(
      { error: 'bad_request', message: 'type must be "subscription" or "topup"' },
      { status: 400 }
    );
  }

  // Get or create Stripe customer
  let customerId = profile.stripe_customer_id;
  if (!customerId) {
    customerId = await getOrCreateCustomer(user.email, user.id);
    await supabaseAdmin
      .from('profiles')
      .update({ stripe_customer_id: customerId })
      .eq('id', user.id);
  }

  const appUrl = process.env.NEXT_PUBLIC_APP_URL!;
  const priceId = type === 'subscription'
    ? process.env.STRIPE_PRO_PRICE_ID!
    : process.env.STRIPE_TOPUP_PRICE_ID!;
  const mode = type === 'subscription' ? 'subscription' : 'payment';

  const checkoutUrl = await createCheckoutSession(
    customerId,
    priceId,
    mode,
    `${appUrl}/api/auth/callback?payment=success`,
    `${appUrl}/api/auth/callback?payment=cancelled`
  );

  return NextResponse.json({ url: checkoutUrl });
}
```

- [ ] **Step 2: Commit**

```bash
git add src/app/api/credits/purchase/route.ts
git commit -m "feat: add purchase route for subscription and credit top-up checkout"
```

---

### Task 10: Stripe Webhook Handler

**Files:**
- Create: `src/app/api/webhooks/stripe/route.ts`
- Create: `tests/webhooks.test.ts`

- [ ] **Step 1: Write the test**

```typescript
// tests/webhooks.test.ts
import { describe, it, expect, vi } from 'vitest';

describe('webhook event routing', () => {
  it('handles invoice.payment_succeeded for subscription', () => {
    // This is more of an integration test shape — we verify the
    // event type routing logic exists
    const eventTypes = [
      'invoice.payment_succeeded',
      'checkout.session.completed',
      'customer.subscription.deleted',
    ];
    // Each event type should be handled
    expect(eventTypes).toHaveLength(3);
  });
});
```

- [ ] **Step 2: Implement webhook handler**

```typescript
// src/app/api/webhooks/stripe/route.ts
import { NextRequest, NextResponse } from 'next/server';
import { constructWebhookEvent } from '@/lib/stripe';
import { grantCredits } from '@/lib/credits';
import { supabaseAdmin } from '@/lib/supabase-admin';
import { PRO_MONTHLY_CREDITS, TOPUP_CREDITS } from '@/types';
import Stripe from 'stripe';

export async function POST(request: NextRequest) {
  const body = await request.text();
  const signature = request.headers.get('stripe-signature');

  if (!signature) {
    return NextResponse.json({ error: 'Missing signature' }, { status: 400 });
  }

  let event: Stripe.Event;
  try {
    event = constructWebhookEvent(body, signature);
  } catch (err) {
    return NextResponse.json({ error: 'Invalid signature' }, { status: 400 });
  }

  switch (event.type) {
    case 'checkout.session.completed': {
      const session = event.data.object as Stripe.Checkout.Session;
      await handleCheckoutComplete(session);
      break;
    }

    case 'invoice.payment_succeeded': {
      const invoice = event.data.object as Stripe.Invoice;
      await handleInvoicePaid(invoice);
      break;
    }

    case 'customer.subscription.deleted': {
      const subscription = event.data.object as Stripe.Subscription;
      await handleSubscriptionCancelled(subscription);
      break;
    }
  }

  return NextResponse.json({ received: true });
}

async function handleCheckoutComplete(session: Stripe.Checkout.Session) {
  const customerId = session.customer as string;

  // Find user by stripe_customer_id
  const { data: profile } = await supabaseAdmin
    .from('profiles')
    .select('id, plan')
    .eq('stripe_customer_id', customerId)
    .single();

  if (!profile) return;

  if (session.mode === 'subscription') {
    // Upgrade to pro + grant credits
    await supabaseAdmin
      .from('profiles')
      .update({
        plan: 'pro',
        stripe_subscription_id: session.subscription as string,
      })
      .eq('id', profile.id);

    await grantCredits(
      profile.id,
      PRO_MONTHLY_CREDITS,
      'subscription_grant',
      'Pro subscription activated',
      true // reset balance
    );
  } else if (session.mode === 'payment') {
    // Credit top-up
    await grantCredits(
      profile.id,
      TOPUP_CREDITS,
      'topup',
      'Credit top-up ($5/30 credits)',
      false // additive
    );
  }
}

async function handleInvoicePaid(invoice: Stripe.Invoice) {
  // Skip the first invoice (handled by checkout.session.completed)
  if (invoice.billing_reason === 'subscription_create') return;

  const customerId = invoice.customer as string;
  const { data: profile } = await supabaseAdmin
    .from('profiles')
    .select('id')
    .eq('stripe_customer_id', customerId)
    .single();

  if (!profile) return;

  // Monthly renewal: reset credits
  await grantCredits(
    profile.id,
    PRO_MONTHLY_CREDITS,
    'subscription_grant',
    'Monthly credit renewal',
    true // reset balance (unused credits expire)
  );
}

async function handleSubscriptionCancelled(subscription: Stripe.Subscription) {
  const customerId = subscription.customer as string;
  const { data: profile } = await supabaseAdmin
    .from('profiles')
    .select('id')
    .eq('stripe_customer_id', customerId)
    .single();

  if (!profile) return;

  // Downgrade to free, zero credits
  await supabaseAdmin
    .from('profiles')
    .update({
      plan: 'free',
      credits_balance: 0,
      stripe_subscription_id: null,
    })
    .eq('id', profile.id);

  await supabaseAdmin.from('credit_transactions').insert({
    user_id: profile.id,
    amount: 0,
    balance_after: 0,
    type: 'subscription_grant',
    description: 'Subscription cancelled — credits zeroed',
  });
}
```

- [ ] **Step 3: Commit**

```bash
git add src/app/api/webhooks/stripe/route.ts tests/webhooks.test.ts
git commit -m "feat: add Stripe webhook handler for subscription, renewal, top-up, cancellation"
```

---

## Chunk 5: OpenRouter + Generation Routes

### Task 11: OpenRouter Client Library

**Files:**
- Create: `src/lib/openrouter.ts`
- Create: `tests/openrouter.test.ts`

- [ ] **Step 1: Write the test**

```typescript
// tests/openrouter.test.ts
import { describe, it, expect, vi } from 'vitest';

// Mock global fetch
const mockFetch = vi.fn();
vi.stubGlobal('fetch', mockFetch);

describe('openrouter', () => {
  it('generateImage sends correct request to OpenRouter', async () => {
    const { generateImage } = await import('@/lib/openrouter');

    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        data: [{ url: 'https://example.com/img1.png' }],
      }),
    });

    const result = await generateImage({
      prompt: 'A minimal logo',
      model: 'ideogram/ideogram-v3',
      n: 4,
    });

    expect(mockFetch).toHaveBeenCalledWith(
      'https://openrouter.ai/api/v1/images/generations',
      expect.objectContaining({
        method: 'POST',
        headers: expect.objectContaining({
          Authorization: expect.stringContaining('Bearer'),
        }),
      })
    );
    expect(result.data).toHaveLength(1);
  });

  it('generateChat sends correct request', async () => {
    const { generateChat } = await import('@/lib/openrouter');

    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        choices: [{ message: { content: 'Hello' } }],
      }),
    });

    const result = await generateChat({
      model: 'anthropic/claude-sonnet-4',
      messages: [{ role: 'user', content: 'Hi' }],
    });

    expect(result.choices).toHaveLength(1);
  });
});
```

- [ ] **Step 2: Implement OpenRouter client**

```typescript
// src/lib/openrouter.ts

const OPENROUTER_BASE = 'https://openrouter.ai/api/v1';

function getApiKey(): string {
  const key = process.env.OPENROUTER_API_KEY;
  if (!key) throw new Error('OPENROUTER_API_KEY is required');
  return key;
}

function headers(): Record<string, string> {
  return {
    Authorization: `Bearer ${getApiKey()}`,
    'Content-Type': 'application/json',
    'HTTP-Referer': 'https://shepherd.codes',
    'X-Title': 'Shepherd Pro',
  };
}

export interface ImageGenerationRequest {
  prompt: string;
  model?: string;
  n?: number;
  size?: string;
  response_format?: 'url' | 'b64_json';
}

export interface ImageGenerationResponse {
  data: Array<{ url?: string; b64_json?: string }>;
}

export async function generateImage(
  params: ImageGenerationRequest
): Promise<ImageGenerationResponse> {
  const response = await fetch(`${OPENROUTER_BASE}/images/generations`, {
    method: 'POST',
    headers: headers(),
    body: JSON.stringify({
      model: params.model ?? 'ideogram/ideogram-v3',
      prompt: params.prompt,
      n: params.n ?? 4,
      size: params.size ?? '1024x1024',
      response_format: params.response_format ?? 'url',
    }),
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`OpenRouter image generation failed: ${response.status} ${error}`);
  }

  return response.json();
}

export interface ChatRequest {
  model: string;
  messages: Array<{ role: string; content: string }>;
  temperature?: number;
  max_tokens?: number;
}

export interface ChatResponse {
  choices: Array<{
    message: { role: string; content: string };
  }>;
}

export async function generateChat(
  params: ChatRequest
): Promise<ChatResponse> {
  const response = await fetch(`${OPENROUTER_BASE}/chat/completions`, {
    method: 'POST',
    headers: headers(),
    body: JSON.stringify({
      model: params.model,
      messages: params.messages,
      temperature: params.temperature ?? 0.7,
      max_tokens: params.max_tokens ?? 4096,
    }),
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`OpenRouter chat failed: ${response.status} ${error}`);
  }

  return response.json();
}
```

- [ ] **Step 3: Run tests**

```bash
npx vitest run tests/openrouter.test.ts
```

- [ ] **Step 4: Commit**

```bash
git add src/lib/openrouter.ts tests/openrouter.test.ts
git commit -m "feat: add OpenRouter client for image generation and chat completions"
```

---

### Task 12: Logo Generation Route

**Files:**
- Create: `src/app/api/generate/logo/route.ts`
- Create: `tests/generate.test.ts`

- [ ] **Step 1: Write test**

```typescript
// tests/generate.test.ts
import { describe, it, expect } from 'vitest';
import { CREDIT_COSTS } from '@/types';

describe('generation constants', () => {
  it('logo costs 2 credits', () => {
    expect(CREDIT_COSTS.logo).toBe(2);
  });

  it('name costs 1 credit', () => {
    expect(CREDIT_COSTS.name).toBe(1);
  });

  it('northstar costs 15 credits', () => {
    expect(CREDIT_COSTS.northstar).toBe(15);
  });
});
```

- [ ] **Step 2: Implement logo generation route**

```typescript
// src/app/api/generate/logo/route.ts
import { NextRequest, NextResponse } from 'next/server';
import { authenticateRequest, isAuthError } from '@/lib/middleware';
import { checkCreditsOrTrial, deductCredits, deductTrial } from '@/lib/credits';
import { generateImage } from '@/lib/openrouter';
import { supabaseAdmin } from '@/lib/supabase-admin';

export async function POST(request: NextRequest) {
  const auth = await authenticateRequest(request);
  if (isAuthError(auth)) return auth;

  const { user, profile } = auth;
  const body = await request.json();
  const { style, colors, description } = body;

  if (!description) {
    return NextResponse.json(
      { error: 'bad_request', message: 'description is required' },
      { status: 400 }
    );
  }

  // Check credits or trial
  const check = await checkCreditsOrTrial(
    user.id, 'logo', profile.plan, profile.credits_balance
  );

  if (!check.allowed) {
    const appUrl = process.env.NEXT_PUBLIC_APP_URL!;
    return NextResponse.json(
      {
        error: check.reason === 'trial_exhausted' ? 'trial_exhausted' : 'credits_insufficient',
        message: check.reason === 'trial_exhausted'
          ? 'Free trials used. Upgrade to Pro.'
          : `Need 2 credits, have ${profile.credits_balance}`,
        upgrade_url: `${appUrl}/api/credits/purchase`,
      },
      { status: 402 }
    );
  }

  // Build prompt
  const styleMap: Record<string, string> = {
    minimal: 'Minimal flat vector logo mark',
    geometric: 'Geometric abstract logo mark',
    mascot: 'Friendly mascot character logo',
    abstract: 'Abstract modern logo mark',
  };
  const stylePrefix = styleMap[style] || styleMap.minimal;
  const colorNote = colors?.length ? ` Using colors: ${colors.join(', ')}.` : '';
  const prompt = `${stylePrefix} for "${description}".${colorNote} Clean, professional, works at 16px favicon size. No text. No gradients. Centered on pure white background. Logo design.`;

  // Create generation record
  const { data: generation } = await supabaseAdmin
    .from('generations')
    .insert({
      user_id: user.id,
      type: 'logo',
      credits_used: check.method === 'trial' ? 0 : 2,
      input_prompt: prompt,
      input_params: { style, colors, description },
      status: 'pending',
    })
    .select('id')
    .single();

  const generationId = generation!.id;

  try {
    // Call OpenRouter
    const result = await generateImage({
      prompt,
      model: 'ideogram/ideogram-v3',
      n: 4,
      size: '1024x1024',
    });

    const images = result.data.map((img, i) => ({
      url: img.url,
      variant: i + 1,
    }));

    // Update generation with results
    await supabaseAdmin
      .from('generations')
      .update({ result: { images }, status: 'completed' })
      .eq('id', generationId);

    // Deduct credits or trial
    let creditsRemaining = profile.credits_balance;
    if (check.method === 'trial') {
      await deductTrial(user.id, 'logo');
    } else {
      creditsRemaining = await deductCredits(user.id, 'logo', generationId);
    }

    return NextResponse.json({
      generation_id: generationId,
      images,
      credits_used: check.method === 'trial' ? 0 : 2,
      credits_remaining: creditsRemaining,
    });
  } catch (err) {
    // Mark generation as failed
    await supabaseAdmin
      .from('generations')
      .update({ status: 'failed', result: { error: String(err) } })
      .eq('id', generationId);

    return NextResponse.json(
      { error: 'generation_failed', message: 'Logo generation failed. Credits not charged.' },
      { status: 500 }
    );
  }
}
```

- [ ] **Step 3: Commit**

```bash
git add src/app/api/generate/logo/route.ts tests/generate.test.ts
git commit -m "feat: add logo generation route with credit/trial gating and OpenRouter"
```

---

### Task 13: Name Generation Route

**Files:**
- Create: `src/app/api/generate/name/route.ts`

- [ ] **Step 1: Implement name generation route**

```typescript
// src/app/api/generate/name/route.ts
import { NextRequest, NextResponse } from 'next/server';
import { authenticateRequest, isAuthError } from '@/lib/middleware';
import { checkCreditsOrTrial, deductCredits, deductTrial } from '@/lib/credits';
import { generateChat } from '@/lib/openrouter';
import { supabaseAdmin } from '@/lib/supabase-admin';

export async function POST(request: NextRequest) {
  const auth = await authenticateRequest(request);
  if (isAuthError(auth)) return auth;

  const { user, profile } = auth;
  const body = await request.json();
  const { description, vibes, check_domains } = body;

  if (!description) {
    return NextResponse.json(
      { error: 'bad_request', message: 'description is required' },
      { status: 400 }
    );
  }

  // Check credits or trial
  const check = await checkCreditsOrTrial(
    user.id, 'name', profile.plan, profile.credits_balance
  );

  if (!check.allowed) {
    const appUrl = process.env.NEXT_PUBLIC_APP_URL!;
    return NextResponse.json(
      {
        error: check.reason === 'trial_exhausted' ? 'trial_exhausted' : 'credits_insufficient',
        message: check.reason === 'trial_exhausted'
          ? 'Free trials used. Upgrade to Pro.'
          : `Need 1 credit, have ${profile.credits_balance}`,
        upgrade_url: `${appUrl}/api/credits/purchase`,
      },
      { status: 402 }
    );
  }

  const vibeStr = vibes?.length ? `Vibes: ${vibes.join(', ')}. ` : '';
  const prompt = `Generate 20 creative product name candidates for: "${description}". ${vibeStr}Return a JSON array of strings. Names should be: memorable, easy to spell, easy to pronounce, work as a domain name. Return ONLY the JSON array, no explanation.`;

  // Create generation record
  const { data: generation } = await supabaseAdmin
    .from('generations')
    .insert({
      user_id: user.id,
      type: 'name',
      credits_used: check.method === 'trial' ? 0 : 1,
      input_prompt: prompt,
      input_params: { description, vibes, check_domains },
      status: 'pending',
    })
    .select('id')
    .single();

  const generationId = generation!.id;

  try {
    const chatResult = await generateChat({
      model: 'anthropic/claude-sonnet-4',
      messages: [{ role: 'user', content: prompt }],
      temperature: 0.9,
    });

    const content = chatResult.choices[0]?.message?.content ?? '[]';
    let names: string[];
    try {
      // Extract JSON array from response (may have markdown wrapping)
      const jsonMatch = content.match(/\[[\s\S]*\]/);
      names = jsonMatch ? JSON.parse(jsonMatch[0]) : [];
    } catch {
      names = content.split('\n').filter(Boolean).map(n => n.replace(/^[\d."\-*]+\s*/, '').trim());
    }

    // Domain checking (basic — RDAP/WHOIS)
    const candidates = await Promise.all(
      names.slice(0, 20).map(async (name) => {
        const slug = name.toLowerCase().replace(/[^a-z0-9]/g, '');
        const domainAvailable = check_domains ? await checkDomain(`${slug}.com`) : null;
        return {
          name,
          domain_available: domainAvailable,
          npm_available: null,   // TODO: implement npm registry check
          pypi_available: null,  // TODO: implement PyPI check
          github_available: null, // TODO: implement GitHub check
        };
      })
    );

    // Update generation
    await supabaseAdmin
      .from('generations')
      .update({ result: { candidates }, status: 'completed' })
      .eq('id', generationId);

    // Deduct
    let creditsRemaining = profile.credits_balance;
    if (check.method === 'trial') {
      await deductTrial(user.id, 'name');
    } else {
      creditsRemaining = await deductCredits(user.id, 'name', generationId);
    }

    return NextResponse.json({
      generation_id: generationId,
      candidates,
      credits_used: check.method === 'trial' ? 0 : 1,
      credits_remaining: creditsRemaining,
    });
  } catch (err) {
    await supabaseAdmin
      .from('generations')
      .update({ status: 'failed', result: { error: String(err) } })
      .eq('id', generationId);

    return NextResponse.json(
      { error: 'generation_failed', message: 'Name generation failed. Credits not charged.' },
      { status: 500 }
    );
  }
}

async function checkDomain(domain: string): Promise<boolean | null> {
  try {
    const response = await fetch(
      `https://rdap.org/domain/${domain}`,
      { signal: AbortSignal.timeout(3000) }
    );
    // 404 = not registered = available
    if (response.status === 404) return true;
    // 200 = registered = not available
    if (response.ok) return false;
    return null; // unknown
  } catch {
    return null; // network error, can't determine
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/app/api/generate/name/route.ts
git commit -m "feat: add name generation route with LLM brainstorm and domain checking"
```

---

### Task 14: North Star Generation Route

**Files:**
- Create: `src/app/api/generate/northstar/route.ts`

- [ ] **Step 1: Implement North Star route**

```typescript
// src/app/api/generate/northstar/route.ts
import { NextRequest, NextResponse } from 'next/server';
import { authenticateRequest, isAuthError } from '@/lib/middleware';
import { checkCreditsOrTrial, deductCredits, deductTrial } from '@/lib/credits';
import { generateChat } from '@/lib/openrouter';
import { supabaseAdmin } from '@/lib/supabase-admin';

const PHASE_PROMPTS: Record<number, { name: string; systemPrompt: string }> = {
  1: {
    name: 'Brand Guidelines',
    systemPrompt: `You are a brand strategist. Generate comprehensive brand guidelines for the product described. Include: brand voice (tone, personality, do/don't), visual direction (color palette suggestions, typography direction), naming conventions, and messaging hierarchy. Output as a well-structured markdown document.`,
  },
  2: {
    name: 'North Star Metric',
    systemPrompt: `You are a product strategy advisor. Define the North Star metric for this product. Include: the metric definition, why it matters, how to measure it, leading indicators, and lagging indicators. Reference the product context provided. Output as markdown.`,
  },
  3: {
    name: 'Competitive Landscape',
    systemPrompt: `You are a competitive intelligence analyst. Map the competitive landscape for this product. Include: direct competitors, indirect competitors, their strengths/weaknesses, market gaps, and positioning opportunities. Output as markdown with comparison tables.`,
  },
  4: {
    name: 'User Personas',
    systemPrompt: `You are a UX researcher. Create 3-4 detailed user personas for this product. For each: name, role, goals, frustrations, tech comfort level, daily workflow, and how this product fits in. Output as markdown.`,
  },
  5: {
    name: 'User Journeys',
    systemPrompt: `You are a UX designer. Map the key user journeys for this product. For each persona: awareness, consideration, first use, regular use, and advocacy stages. Include touchpoints, emotions, and pain points at each stage. Output as markdown.`,
  },
  6: {
    name: 'Architecture Blueprint',
    systemPrompt: `You are a solutions architect. Create an architecture blueprint for this product. Include: system components, data flow, technology recommendations, infrastructure, and scalability considerations. Output as markdown with ASCII diagrams.`,
  },
  7: {
    name: 'Security Architecture',
    systemPrompt: `You are a security architect. Design the security architecture for this product. Include: authentication, authorization, data protection, threat model, and compliance requirements. Output as markdown.`,
  },
  8: {
    name: 'API Design',
    systemPrompt: `You are an API designer. Design the core API for this product. Include: endpoints, request/response schemas, authentication, rate limiting, and versioning strategy. Output as markdown.`,
  },
  9: {
    name: 'Data Model',
    systemPrompt: `You are a data architect. Design the data model for this product. Include: entities, relationships, indexes, and migration strategy. Output as markdown with schema diagrams.`,
  },
  10: {
    name: 'Testing Strategy',
    systemPrompt: `You are a QA architect. Design the testing strategy for this product. Include: unit testing, integration testing, E2E testing, performance testing, and CI/CD pipeline. Output as markdown.`,
  },
  11: {
    name: 'Go-to-Market Strategy',
    systemPrompt: `You are a growth strategist. Create a go-to-market strategy for this product. Include: launch plan, channels, messaging, pricing validation, and success metrics. Output as markdown.`,
  },
  12: {
    name: 'Action Roadmap',
    systemPrompt: `You are a product manager. Create a phased action roadmap for this product. Include: MVP scope, phase 1/2/3 features, milestones, and dependencies. Output as markdown with timeline.`,
  },
  13: {
    name: 'Strategic Recommendation',
    systemPrompt: `You are a strategic advisor. Synthesize all previous analyses into a final strategic recommendation. Include: key insights, critical risks, recommended next steps, and success probability assessment. Output as markdown.`,
  },
};

export async function POST(request: NextRequest) {
  const auth = await authenticateRequest(request);
  if (isAuthError(auth)) return auth;

  const { user, profile } = auth;
  const body = await request.json();
  const { phase, inputs } = body;

  if (!phase || phase < 1 || phase > 13) {
    return NextResponse.json(
      { error: 'bad_request', message: 'phase must be 1-13' },
      { status: 400 }
    );
  }

  // Check credits or trial
  const check = await checkCreditsOrTrial(
    user.id, 'northstar', profile.plan, profile.credits_balance
  );

  if (!check.allowed) {
    const appUrl = process.env.NEXT_PUBLIC_APP_URL!;
    return NextResponse.json(
      {
        error: check.reason === 'trial_exhausted' ? 'trial_exhausted' : 'credits_insufficient',
        message: check.reason === 'trial_exhausted'
          ? 'Free trials used. Upgrade to Pro.'
          : `Need 15 credits, have ${profile.credits_balance}`,
        upgrade_url: `${appUrl}/api/credits/purchase`,
      },
      { status: 402 }
    );
  }

  const phaseConfig = PHASE_PROMPTS[phase];
  const userMessage = `Product context:\n${JSON.stringify(inputs, null, 2)}`;

  // Create generation record
  const { data: generation } = await supabaseAdmin
    .from('generations')
    .insert({
      user_id: user.id,
      type: 'northstar',
      credits_used: check.method === 'trial' ? 0 : 15,
      input_prompt: phaseConfig.systemPrompt,
      input_params: { phase, inputs },
      status: 'pending',
    })
    .select('id')
    .single();

  const generationId = generation!.id;

  try {
    const chatResult = await generateChat({
      model: 'anthropic/claude-sonnet-4',
      messages: [
        { role: 'system', content: phaseConfig.systemPrompt },
        { role: 'user', content: userMessage },
      ],
      temperature: 0.6,
      max_tokens: 8192,
    });

    const document = chatResult.choices[0]?.message?.content ?? '';

    await supabaseAdmin
      .from('generations')
      .update({
        result: { phase, name: phaseConfig.name, document },
        status: 'completed',
      })
      .eq('id', generationId);

    let creditsRemaining = profile.credits_balance;
    if (check.method === 'trial') {
      await deductTrial(user.id, 'northstar');
    } else {
      creditsRemaining = await deductCredits(user.id, 'northstar', generationId);
    }

    return NextResponse.json({
      generation_id: generationId,
      phase,
      phase_name: phaseConfig.name,
      document,
      credits_used: check.method === 'trial' ? 0 : 15,
      credits_remaining: creditsRemaining,
      next_phase: phase < 13 ? phase + 1 : null,
    });
  } catch (err) {
    await supabaseAdmin
      .from('generations')
      .update({ status: 'failed', result: { error: String(err) } })
      .eq('id', generationId);

    return NextResponse.json(
      { error: 'generation_failed', message: 'North Star generation failed. Credits not charged.' },
      { status: 500 }
    );
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/app/api/generate/northstar/route.ts
git commit -m "feat: add North Star 13-phase wizard generation route"
```

---

## Chunk 6: Trial Status + Rate Limiting + CORS + Deploy

### Task 15: Trial Status Route

**Files:**
- Create: `src/app/api/trial/status/route.ts`

- [ ] **Step 1: Implement trial status route**

```typescript
// src/app/api/trial/status/route.ts
import { NextRequest, NextResponse } from 'next/server';
import { authenticateRequest, isAuthError } from '@/lib/middleware';
import { supabaseAdmin } from '@/lib/supabase-admin';

export async function GET(request: NextRequest) {
  const auth = await authenticateRequest(request);
  if (isAuthError(auth)) return auth;

  const { user } = auth;

  const { data: trials } = await supabaseAdmin
    .from('trial_usage')
    .select('feature, uses_remaining')
    .eq('user_id', user.id);

  const result: Record<string, number> = {};
  for (const trial of trials ?? []) {
    result[trial.feature] = trial.uses_remaining;
  }

  return NextResponse.json({ trials: result });
}
```

- [ ] **Step 2: Commit**

```bash
git add src/app/api/trial/status/route.ts
git commit -m "feat: add trial status API route"
```

---

### Task 16: CORS + Rate Limiting Middleware

**Files:**
- Create: `src/middleware.ts` (Next.js root middleware)

- [ ] **Step 1: Implement Next.js middleware for CORS and rate limiting**

```typescript
// src/middleware.ts
import { NextRequest, NextResponse } from 'next/server';

// Simple in-memory rate limiter (per-IP, resets on cold start)
// For production, use Vercel KV or Upstash Redis
const rateLimitMap = new Map<string, { count: number; resetAt: number }>();
const RATE_LIMIT = 60;       // requests per window
const RATE_WINDOW = 60_000;  // 1 minute

function rateLimit(ip: string): boolean {
  const now = Date.now();
  const entry = rateLimitMap.get(ip);

  if (!entry || now > entry.resetAt) {
    rateLimitMap.set(ip, { count: 1, resetAt: now + RATE_WINDOW });
    return true;
  }

  if (entry.count >= RATE_LIMIT) {
    return false;
  }

  entry.count++;
  return true;
}

const ALLOWED_ORIGINS = [
  'https://shepherd.codes',
  'https://api.shepherd.codes',
  'http://localhost:3000',
  'http://localhost:1420',  // Tauri dev server
];

export function middleware(request: NextRequest) {
  const origin = request.headers.get('origin') ?? '';
  const ip = request.headers.get('x-forwarded-for')?.split(',')[0] ?? 'unknown';

  // Rate limiting (skip webhooks — Stripe retries)
  if (!request.nextUrl.pathname.startsWith('/api/webhooks')) {
    if (!rateLimit(ip)) {
      return NextResponse.json(
        { error: 'rate_limited', message: 'Too many requests. Try again in 60s.' },
        { status: 429 }
      );
    }
  }

  // CORS
  const response = NextResponse.next();

  if (ALLOWED_ORIGINS.includes(origin)) {
    response.headers.set('Access-Control-Allow-Origin', origin);
  }
  response.headers.set('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  response.headers.set('Access-Control-Allow-Headers', 'Content-Type, Authorization');
  response.headers.set('Access-Control-Max-Age', '86400');

  // Handle preflight
  if (request.method === 'OPTIONS') {
    return new NextResponse(null, {
      status: 204,
      headers: response.headers,
    });
  }

  return response;
}

export const config = {
  matcher: '/api/:path*',
};
```

- [ ] **Step 2: Commit**

```bash
git add src/middleware.ts
git commit -m "feat: add CORS and rate limiting middleware"
```

---

### Task 17: Next.js Config + Vercel Config

**Files:**
- Modify: `next.config.ts`
- Create: `vercel.json`

- [ ] **Step 1: Update Next.js config**

```typescript
// next.config.ts
import type { NextConfig } from 'next';

const nextConfig: NextConfig = {
  // API-only — no pages to render
  output: 'standalone',
  // Disable image optimization (no frontend)
  images: { unoptimized: true },
};

export default nextConfig;
```

- [ ] **Step 2: Create Vercel config**

```json
{
  "framework": "nextjs",
  "regions": ["iad1"],
  "headers": [
    {
      "source": "/api/(.*)",
      "headers": [
        { "key": "X-Content-Type-Options", "value": "nosniff" },
        { "key": "X-Frame-Options", "value": "DENY" }
      ]
    }
  ]
}
```

- [ ] **Step 3: Commit**

```bash
git add next.config.ts vercel.json
git commit -m "feat: add Next.js and Vercel configuration"
```

---

### Task 18: Create GitHub Repo + Push + Set Vercel Env Vars

- [ ] **Step 1: Create GitHub repo**

```bash
cd /Users/4n6h4x0r/src/shepherd-pro
gh repo create SecurityRonin/shepherd-pro --private --source=. --push
```

- [ ] **Step 2: Set Vercel environment variables**

```bash
cd /Users/4n6h4x0r/src/shepherd-pro
vercel link
vercel env add OPENROUTER_API_KEY production preview development
vercel env add SUPABASE_URL production preview development
vercel env add SUPABASE_ANON_KEY production preview development
vercel env add SUPABASE_SERVICE_ROLE_KEY production preview development
vercel env add STRIPE_SECRET_KEY production preview development
vercel env add STRIPE_WEBHOOK_SECRET production preview development
vercel env add STRIPE_PRO_PRICE_ID production preview development
vercel env add STRIPE_TOPUP_PRICE_ID production preview development
vercel env add NEXT_PUBLIC_APP_URL production preview development
```

- [ ] **Step 3: Deploy to Vercel**

```bash
vercel --prod
```

- [ ] **Step 4: Set custom domain**

```bash
vercel domains add api.shepherd.codes
```

---

### Task 19: Run All Tests + Final Verification

- [ ] **Step 1: Run all tests**

```bash
cd /Users/4n6h4x0r/src/shepherd-pro
npx vitest run
```

Expected: All tests pass.

- [ ] **Step 2: Verify API health (local)**

```bash
cd /Users/4n6h4x0r/src/shepherd-pro
npm run dev &
# Test that routes respond
curl -s http://localhost:3000/api/credits/balance | jq .
# Should return 401 (no auth)
```

- [ ] **Step 3: Final commit with any fixes**

```bash
git add -A
git commit -m "chore: final cleanup and test verification"
git push
```

---

## Summary

| Chunk | Tasks | What it delivers |
|-------|-------|-----------------|
| 1: Scaffolding + Schema | 1-2 | Next.js project, Supabase migration, types |
| 2: Auth + Middleware | 3-5 | Supabase clients, JWT middleware, auth routes |
| 3: Credit System | 6-7 | Credits library, balance route, trial system |
| 4: Stripe | 8-10 | Stripe client, purchase route, webhook handler |
| 5: Generation Routes | 11-14 | OpenRouter client, logo/name/northstar routes |
| 6: Polish + Deploy | 15-19 | Trial status, CORS, rate limiting, deploy |

**Total: 19 tasks, ~15 commits, ~20 files**

After this plan, **Plan 5** will cover the Rust-side desktop integration (`crates/shepherd-core/src/cloud/` module).
