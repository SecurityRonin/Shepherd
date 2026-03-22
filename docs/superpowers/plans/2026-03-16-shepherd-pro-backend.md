# Shepherd Pro Backend Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Build the `SecurityRonin/shepherd-pro` Next.js backend deployed at `api.shepherd.codes` providing auth, credit management, Stripe payments, and 8 generative AI endpoints for the Shepherd desktop app.

**Architecture:** Next.js 15 App Router API routes on Vercel, Supabase for auth + Postgres with RLS, Stripe for subscriptions and top-ups. All 8 generation routes follow the same middleware pipeline: verifyJwt → checkTrialOrDeductCredits → callUpstream → saveGeneration → respond.

**Tech Stack:** Next.js 15, TypeScript, Supabase JS v2, Stripe Node SDK, Vitest, MSW (Mock Service Worker), Upstash Ratelimit, OpenRouter, Firecrawl, Exa.

**Spec:** `docs/superpowers/specs/2026-03-16-shepherd-pro-backend-design.md` (in the `SecurityRonin/shepherd` repo)

---

## Chunk 1: Repo Scaffold + Test Infrastructure

## Chunk 2: Database Migrations + Core Libs

## Chunk 3: Auth + Credits Routes

## Chunk 4: Generation Routes — Logo, Name, North Star

## Chunk 5: Generation Routes — Scrape, Crawl, Vision, Search

## Chunk 6: Stripe Webhook

## Chunk 7: Desktop App Update + Deployment

---

## Chunk 1: Repo Scaffold + Test Infrastructure

### Task 1: Create and initialise the repo

**Files:**
- Create: `package.json` (via Next.js scaffold)
- Create: `tsconfig.json`
- Create: `.env.example`
- Create: `.gitignore`
- Create: `vitest.config.ts`
- Create: `__tests__/setup.ts`

- [x] **Step 1: Create the GitHub repo**

```bash
gh repo create SecurityRonin/shepherd-pro --public --description "Shepherd Pro backend — api.shepherd.codes"
cd ~/src
git clone https://github.com/SecurityRonin/shepherd-pro.git
cd shepherd-pro
```

- [x] **Step 2: Scaffold Next.js 15**

```bash
npx create-next-app@latest . \
  --typescript \
  --no-tailwind \
  --no-eslint \
  --app \
  --no-src-dir \
  --import-alias "@/*"
```

When prompted: say yes to use `src/` directory if asked; accept defaults otherwise.

- [x] **Step 3: Install runtime dependencies**

```bash
npm install \
  @supabase/supabase-js \
  stripe \
  @upstash/ratelimit \
  @upstash/redis \
  @firecrawl/js \
  exa-js
```

- [x] **Step 4: Install dev dependencies**

```bash
npm install -D vitest @vitest/coverage-v8 msw tsx
```

- [x] **Step 5: Create `.env.example`**

Create file `shepherd-pro/.env.example`:
```bash
# OpenRouter — covers logo (ideogram/ideogram-v2), name (claude-sonnet-4-6),
#              northstar (claude-opus-4-6), vision (claude-sonnet-4-6)
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
STRIPE_PRO_PRICE_ID=price_...
STRIPE_TOPUP_PRICE_ID=price_...

# Upstash Redis — rate limiting
UPSTASH_REDIS_REST_URL=https://xxx.upstash.io
UPSTASH_REDIS_REST_TOKEN=AXxx...

# App
NEXT_PUBLIC_APP_URL=https://api.shepherd.codes
```

Copy to `.env.local` and fill in real values.

- [x] **Step 6: Create `vitest.config.ts`**

```ts
import { defineConfig } from 'vitest/config'
export default defineConfig({
  test: {
    environment: 'node',
    globals: true,
    setupFiles: ['__tests__/setup.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'lcov'],
    },
  },
})
```

- [x] **Step 7: Create `__tests__/setup.ts`**

```ts
import { setupServer } from 'msw/node'
import { http, HttpResponse } from 'msw'

export const server = setupServer()
beforeAll(() => server.listen({ onUnhandledRequest: 'warn' }))
afterEach(() => server.resetHandlers())
afterAll(() => server.close())
```

- [x] **Step 8: Add test script to `package.json`**

Add to `"scripts"` in `package.json`:
```json
"test": "vitest run",
"test:watch": "vitest",
"test:coverage": "vitest run --coverage"
```

- [x] **Step 9: Write a smoke test and verify it passes**

Create `__tests__/smoke.test.ts`:
```ts
describe('smoke', () => {
  it('test infrastructure works', () => {
    expect(1 + 1).toBe(2)
  })
})
```

Run: `npm test`
Expected: 1 test passing.

- [x] **Step 10: Delete smoke test and commit**

```bash
rm __tests__/smoke.test.ts
git add -A
git commit -m "feat: scaffold Next.js 15 project with Vitest + MSW"
```

---

## Chunk 2: Database Migrations + Core Libs

### Task 2: Supabase migrations

**Files:**
- Create: `supabase/config.toml`
- Create: `supabase/migrations/001_initial_schema.sql`

- [x] **Step 1: Install Supabase CLI and initialise**

```bash
npm install -D supabase
npx supabase init
```

- [x] **Step 2: Create `supabase/migrations/001_initial_schema.sql`**

```sql
-- profiles: extends Supabase auth.users
create table public.profiles (
  id                     uuid primary key references auth.users(id) on delete cascade,
  email                  text not null,
  github_handle          text,
  plan                   text not null default 'free',
  stripe_customer_id     text,
  stripe_subscription_id text,
  credits_balance        integer not null default 0,
  created_at             timestamptz not null default now(),
  updated_at             timestamptz not null default now()
);

-- credit_transactions: append-only ledger
create table public.credit_transactions (
  id            uuid primary key default gen_random_uuid(),
  user_id       uuid not null references public.profiles(id) on delete cascade,
  amount        integer not null,
  balance_after integer not null,
  type          text not null,
  description   text,
  generation_id uuid,
  created_at    timestamptz not null default now()
);

-- generations: history of all AI calls
create table public.generations (
  id           uuid primary key default gen_random_uuid(),
  user_id      uuid not null references public.profiles(id) on delete cascade,
  type         text not null,
  credits_used integer not null,
  input_params jsonb,
  result       jsonb,
  status       text not null default 'pending',
  created_at   timestamptz not null default now()
);

-- trial_usage: 2 free uses per feature per user
create table public.trial_usage (
  id             uuid primary key default gen_random_uuid(),
  user_id        uuid not null references public.profiles(id) on delete cascade,
  feature        text not null,
  uses_remaining integer not null default 2,
  unique (user_id, feature)
);

-- crawl_jobs: async Firecrawl jobs
create table public.crawl_jobs (
  id                  uuid primary key default gen_random_uuid(),
  user_id             uuid not null references public.profiles(id) on delete cascade,
  firecrawl_crawl_id  text not null,
  status              text not null default 'pending',
  generation_id       uuid references public.generations(id),
  created_at          timestamptz not null default now()
);

-- Indexes
create index idx_credit_transactions_user_id on public.credit_transactions(user_id);
create index idx_generations_user_id         on public.generations(user_id);
create index idx_trial_usage_user_id         on public.trial_usage(user_id);
create index idx_crawl_jobs_user_id          on public.crawl_jobs(user_id);

-- Row Level Security
alter table public.profiles            enable row level security;
alter table public.credit_transactions enable row level security;
alter table public.generations         enable row level security;
alter table public.trial_usage         enable row level security;
alter table public.crawl_jobs          enable row level security;

create policy "profiles_select_self"  on public.profiles            for select using (auth.uid() = id);
create policy "txns_select_self"      on public.credit_transactions for select using (auth.uid() = user_id);
create policy "gen_select_self"       on public.generations         for select using (auth.uid() = user_id);
create policy "trial_select_self"     on public.trial_usage         for select using (auth.uid() = user_id);
create policy "crawl_select_self"     on public.crawl_jobs          for select using (auth.uid() = user_id);

-- Atomic credit deduction (row-locks profile, raises if insufficient)
create or replace function public.deduct_credits(
  p_user_id     uuid,
  p_amount      int,
  p_description text
) returns int language plpgsql security definer as $$
declare
  v_balance int;
begin
  select credits_balance into v_balance
  from public.profiles
  where id = p_user_id
  for update;

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

-- Auto-create profile on signup
create or replace function public.handle_new_user()
returns trigger language plpgsql security definer as $$
begin
  insert into public.profiles (id, email, github_handle)
  values (
    new.id,
    new.email,
    new.raw_user_meta_data->>'user_name'
  );
  return new;
end;
$$;

create trigger on_auth_user_created
  after insert on auth.users
  for each row execute procedure public.handle_new_user();
```

- [x] **Step 3: Apply migration to local Supabase (optional for CI)**

```bash
npx supabase db push
```

Skip if not running Supabase locally. CI will use the hosted project.

- [x] **Step 4: Commit**

```bash
git add supabase/
git commit -m "feat: add Supabase migrations — schema, RLS, deduct_credits fn"
```

---

### Task 3: Core lib — `src/lib/supabase.ts`

**Files:**
- Create: `src/lib/supabase.ts`
- Create: `__tests__/lib/supabase.test.ts`

- [x] **Step 1: Write failing tests**

Create `__tests__/lib/supabase.test.ts`:
```ts
import { describe, it, expect } from 'vitest'
import { getAdminClient, verifyJwt } from '@/lib/supabase'

describe('supabase lib', () => {
  it('getAdminClient returns a supabase client', () => {
    process.env.SUPABASE_URL = 'https://test.supabase.co'
    process.env.SUPABASE_SERVICE_ROLE_KEY = 'service-key'
    const client = getAdminClient()
    expect(client).toBeDefined()
    expect(typeof client.from).toBe('function')
  })

  it('verifyJwt returns null for missing auth header', async () => {
    const result = await verifyJwt(undefined)
    expect(result).toBeNull()
  })
})
```

- [x] **Step 2: Run tests — expect FAIL**

```bash
npm test __tests__/lib/supabase.test.ts
```

Expected: FAIL — `Cannot find module '@/lib/supabase'`

- [x] **Step 3: Create `src/lib/supabase.ts`**

```ts
import { createClient, SupabaseClient } from '@supabase/supabase-js'

let adminClient: SupabaseClient | null = null

export function getAdminClient(): SupabaseClient {
  if (!adminClient) {
    adminClient = createClient(
      process.env.SUPABASE_URL!,
      process.env.SUPABASE_SERVICE_ROLE_KEY!,
      { auth: { autoRefreshToken: false, persistSession: false } }
    )
  }
  return adminClient
}

export function getAnonClient(): SupabaseClient {
  return createClient(
    process.env.SUPABASE_URL!,
    process.env.SUPABASE_ANON_KEY!,
    { auth: { autoRefreshToken: false, persistSession: false } }
  )
}

/** Verify a Bearer JWT. Returns the user object or null. */
export async function verifyJwt(
  authHeader: string | undefined
): Promise<{ id: string; email: string } | null> {
  if (!authHeader?.startsWith('Bearer ')) return null
  const token = authHeader.slice(7)
  const client = getAnonClient()
  const { data, error } = await client.auth.getUser(token)
  if (error || !data.user) return null
  return { id: data.user.id, email: data.user.email ?? '' }
}
```

- [x] **Step 4: Run tests — expect PASS**

```bash
npm test __tests__/lib/supabase.test.ts
```

Expected: 2 passing.

- [x] **Step 5: Commit**

```bash
git add src/lib/supabase.ts __tests__/lib/supabase.test.ts
git commit -m "feat: add Supabase admin/anon clients and verifyJwt helper"
```

---

### Task 4: Core lib — `src/lib/credits.ts`

**Files:**
- Create: `src/lib/credits.ts`
- Create: `__tests__/lib/credits.test.ts`

- [x] **Step 1: Write failing tests**

Create `__tests__/lib/credits.test.ts`:
```ts
import { describe, it, expect, vi } from 'vitest'

// Mock the supabase admin client
vi.mock('@/lib/supabase', () => ({
  getAdminClient: vi.fn(),
}))

import { checkTrial, spendTrial, deductCredits } from '@/lib/credits'
import { getAdminClient } from '@/lib/supabase'

describe('credits lib', () => {
  it('checkTrial returns uses_remaining for existing row', async () => {
    const mockSelect = vi.fn().mockResolvedValue({
      data: [{ uses_remaining: 2 }], error: null
    })
    ;(getAdminClient as any).mockReturnValue({
      from: () => ({ select: () => ({ eq: () => ({ eq: () => ({ limit: () => mockSelect() }) }) }) })
    })
    const result = await checkTrial('user-1', 'logo')
    expect(result).toBe(2)
  })

  it('checkTrial returns default 2 when no row exists', async () => {
    const mockSelect = vi.fn().mockResolvedValue({ data: [], error: null })
    ;(getAdminClient as any).mockReturnValue({
      from: () => ({ select: () => ({ eq: () => ({ eq: () => ({ limit: () => mockSelect() }) }) }) })
    })
    const result = await checkTrial('user-1', 'logo')
    expect(result).toBe(2)
  })
})
```

- [x] **Step 2: Run tests — expect FAIL**

```bash
npm test __tests__/lib/credits.test.ts
```

Expected: FAIL — `Cannot find module '@/lib/credits'`

- [x] **Step 3: Create `src/lib/credits.ts`**

```ts
import { getAdminClient } from './supabase'

const TRIAL_LIMIT = 2

/** Returns uses_remaining for the given user+feature. Defaults to TRIAL_LIMIT if no row. */
export async function checkTrial(userId: string, feature: string): Promise<number> {
  const db = getAdminClient()
  const { data } = await db
    .from('trial_usage')
    .select('uses_remaining')
    .eq('user_id', userId)
    .eq('feature', feature)
    .limit(1)
  if (!data || data.length === 0) return TRIAL_LIMIT
  return data[0].uses_remaining
}

/** Decrements trial uses by 1. Inserts row if first use. Returns new remaining count. */
export async function spendTrial(userId: string, feature: string): Promise<number> {
  const db = getAdminClient()
  const current = await checkTrial(userId, feature)
  const next = Math.max(0, current - 1)
  await db
    .from('trial_usage')
    .upsert(
      { user_id: userId, feature, uses_remaining: next },
      { onConflict: 'user_id,feature' }
    )
  return next
}

/** Atomically deduct credits via the deduct_credits Postgres function.
 *  Throws with message "insufficient_credits:..." if balance is too low. */
export async function deductCredits(
  userId: string,
  amount: number,
  description: string
): Promise<number> {
  const db = getAdminClient()
  const { data, error } = await db.rpc('deduct_credits', {
    p_user_id: userId,
    p_amount: amount,
    p_description: description,
  })
  if (error) throw new Error(error.message)
  return data as number
}

/** Save a generation record and return its ID. */
export async function saveGeneration(
  userId: string,
  type: string,
  creditsUsed: number,
  inputParams: object,
  result: object
): Promise<string> {
  const db = getAdminClient()
  const { data, error } = await db
    .from('generations')
    .insert({
      user_id: userId,
      type,
      credits_used: creditsUsed,
      input_params: inputParams,
      result,
      status: 'complete',
    })
    .select('id')
    .single()
  if (error) throw new Error(error.message)
  return data.id
}
```

- [x] **Step 4: Run tests — expect PASS**

```bash
npm test __tests__/lib/credits.test.ts
```

Expected: 2 passing.

- [x] **Step 5: Commit**

```bash
git add src/lib/credits.ts __tests__/lib/credits.test.ts
git commit -m "feat: add credits lib — checkTrial, spendTrial, deductCredits, saveGeneration"
```

---

### Task 5: Core lib — middleware + rate limiter

**Files:**
- Create: `src/lib/middleware.ts`
- Create: `__tests__/lib/middleware.test.ts`

- [x] **Step 1: Write failing tests**

Create `__tests__/lib/middleware.test.ts`:
```ts
import { describe, it, expect, vi } from 'vitest'
import { NextRequest } from 'next/server'

vi.mock('@/lib/supabase', () => ({ verifyJwt: vi.fn() }))
vi.mock('@/lib/credits', () => ({
  checkTrial: vi.fn(),
  spendTrial: vi.fn(),
  deductCredits: vi.fn(),
}))

import { authenticate } from '@/lib/middleware'
import { verifyJwt } from '@/lib/supabase'

function makeRequest(auth?: string) {
  return new NextRequest('http://localhost/api/test', {
    headers: auth ? { authorization: auth } : {},
  })
}

describe('authenticate middleware', () => {
  it('returns 401 when no auth header', async () => {
    ;(verifyJwt as any).mockResolvedValue(null)
    const result = await authenticate(makeRequest())
    expect(result.status).toBe(401)
  })

  it('returns user when JWT valid', async () => {
    ;(verifyJwt as any).mockResolvedValue({ id: 'u1', email: 'a@b.com' })
    const result = await authenticate(makeRequest('Bearer valid'))
    expect('user' in result).toBe(true)
    if ('user' in result) expect(result.user.id).toBe('u1')
  })
})
```

- [x] **Step 2: Run tests — expect FAIL**

```bash
npm test __tests__/lib/middleware.test.ts
```

- [x] **Step 3: Create `src/lib/middleware.ts`**

```ts
import { NextRequest, NextResponse } from 'next/server'
import { Ratelimit } from '@upstash/ratelimit'
import { Redis } from '@upstash/redis'
import { verifyJwt } from './supabase'

let ratelimit: Ratelimit | null = null

function getRatelimit() {
  if (!ratelimit) {
    ratelimit = new Ratelimit({
      redis: new Redis({
        url: process.env.UPSTASH_REDIS_REST_URL!,
        token: process.env.UPSTASH_REDIS_REST_TOKEN!,
      }),
      limiter: Ratelimit.slidingWindow(10, '1 m'),
      prefix: 'shepherd',
    })
  }
  return ratelimit
}

type AuthResult =
  | { user: { id: string; email: string } }
  | NextResponse

/** Verify JWT and return the user, or return a 401 NextResponse. */
export async function authenticate(req: NextRequest): Promise<AuthResult> {
  const user = await verifyJwt(req.headers.get('authorization') ?? undefined)
  if (!user) {
    return NextResponse.json({ error: 'auth_required' }, { status: 401 })
  }
  return { user }
}

/** Rate-limit by user ID. Returns 429 NextResponse or null (allowed). */
export async function rateLimit(
  userId: string,
  endpoint: string
): Promise<NextResponse | null> {
  if (!process.env.UPSTASH_REDIS_REST_URL) return null // skip in dev/test
  const rl = getRatelimit()
  const { success } = await rl.limit(`${userId}:${endpoint}`)
  if (!success) {
    return NextResponse.json(
      { error: 'rate_limited', retry_after: 60 },
      { status: 429 }
    )
  }
  return null
}

export function errorResponse(status: number, error: string, extra?: object) {
  return NextResponse.json({ error, ...extra }, { status })
}
```

- [x] **Step 4: Run tests — expect PASS**

```bash
npm test __tests__/lib/middleware.test.ts
```

Expected: 2 passing.

- [x] **Step 5: Commit**

```bash
git add src/lib/middleware.ts __tests__/lib/middleware.test.ts
git commit -m "feat: add authenticate + rateLimit middleware helpers"
```

---

### Task 6: Core libs — OpenRouter, Firecrawl, Exa clients

**Files:**
- Create: `src/lib/openrouter.ts`
- Create: `src/lib/firecrawl.ts`
- Create: `src/lib/exa.ts`
- Create: `__tests__/lib/openrouter.test.ts`

- [x] **Step 1: Write failing test for OpenRouter**

Create `__tests__/lib/openrouter.test.ts`:
```ts
import { describe, it, expect, beforeEach } from 'vitest'
import { server } from '../setup'
import { http, HttpResponse } from 'msw'
import { chatCompletion, imageGeneration } from '@/lib/openrouter'

const OR_BASE = 'https://openrouter.ai/api/v1'

describe('openrouter lib', () => {
  beforeEach(() => {
    process.env.OPENROUTER_API_KEY = 'test-key'
  })

  it('chatCompletion returns message content', async () => {
    server.use(
      http.post(`${OR_BASE}/chat/completions`, () =>
        HttpResponse.json({
          choices: [{ message: { content: 'Hello world' } }]
        })
      )
    )
    const result = await chatCompletion('anthropic/claude-sonnet-4-6', [
      { role: 'user', content: 'hi' }
    ])
    expect(result).toBe('Hello world')
  })

  it('chatCompletion throws on upstream error', async () => {
    server.use(
      http.post(`${OR_BASE}/chat/completions`, () =>
        HttpResponse.json({ error: 'bad request' }, { status: 400 })
      )
    )
    await expect(
      chatCompletion('anthropic/claude-sonnet-4-6', [])
    ).rejects.toThrow()
  })
})
```

- [x] **Step 2: Run test — expect FAIL**

```bash
npm test __tests__/lib/openrouter.test.ts
```

- [x] **Step 3: Create `src/lib/openrouter.ts`**

```ts
const BASE_URL = 'https://openrouter.ai/api/v1'

type Message = { role: 'system' | 'user' | 'assistant'; content: string | object[] }

async function post(path: string, body: object): Promise<unknown> {
  const res = await fetch(`${BASE_URL}${path}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${process.env.OPENROUTER_API_KEY}`,
      'HTTP-Referer': process.env.NEXT_PUBLIC_APP_URL ?? 'https://api.shepherd.codes',
    },
    body: JSON.stringify(body),
  })
  if (!res.ok) {
    const text = await res.text()
    throw new Error(`OpenRouter ${res.status}: ${text}`)
  }
  return res.json()
}

/** Chat completion — returns the text content of the first choice. */
export async function chatCompletion(
  model: string,
  messages: Message[],
  temperature = 0.7
): Promise<string> {
  const data = await post('/chat/completions', { model, messages, temperature }) as any
  return data.choices[0].message.content as string
}

/** Image generation via OpenRouter (Ideogram). Returns array of image URLs. */
export async function imageGeneration(
  model: string,
  prompt: string,
  n: number
): Promise<string[]> {
  const data = await post('/images/generations', { model, prompt, n }) as any
  return (data.data as Array<{ url: string }>).map((d) => d.url)
}

/** Build a vision message with image_url or base64. */
export function visionMessage(
  prompt: string,
  imageUrl?: string,
  imageBase64?: string
): Message {
  const imageContent = imageUrl
    ? { type: 'image_url', image_url: { url: imageUrl } }
    : { type: 'image_url', image_url: { url: `data:image/png;base64,${imageBase64}` } }
  return {
    role: 'user',
    content: [imageContent, { type: 'text', text: prompt }],
  }
}
```

- [x] **Step 4: Create `src/lib/firecrawl.ts`**

```ts
import FirecrawlApp from '@firecrawl/js'

function getClient() {
  return new FirecrawlApp({ apiKey: process.env.FIRECRAWL_API_KEY! })
}

export interface ScrapeResult {
  markdown?: string
  links: string[]
  metadata: object
}

export async function scrapePage(url: string, formats?: string[]): Promise<ScrapeResult> {
  const client = getClient()
  const result = await client.scrapeUrl(url, {
    formats: (formats as any) ?? ['markdown', 'links'],
  }) as any
  if (!result.success) throw new Error(result.error ?? 'Firecrawl scrape failed')
  return {
    markdown: result.markdown,
    links: result.links ?? [],
    metadata: result.metadata ?? {},
  }
}

export interface CrawlJobResult {
  crawlId: string
}

export async function startCrawl(
  url: string,
  maxDepth?: number,
  limit?: number
): Promise<CrawlJobResult> {
  const client = getClient()
  const result = await client.asyncCrawlUrl(url, {
    maxDepth,
    limit,
  }) as any
  if (!result.success) throw new Error(result.error ?? 'Firecrawl crawl failed')
  return { crawlId: result.id }
}

export async function getCrawlStatus(crawlId: string) {
  const client = getClient()
  const result = await client.checkCrawlStatus(crawlId) as any
  return {
    success: result.success ?? true,
    status: result.status ?? 'unknown',
    total: result.total ?? 0,
    completed: result.completed ?? 0,
    data: (result.data ?? []).map((page: any) => ({
      markdown: page.markdown,
      metadata: page.metadata ?? {},
    })),
  }
}
```

- [x] **Step 5: Create `src/lib/exa.ts`**

```ts
import Exa from 'exa-js'

function getClient() {
  return new Exa(process.env.EXA_API_KEY!)
}

export interface ExaSearchOptions {
  searchType?: 'neural' | 'keyword' | 'auto'
  numResults?: number
  includeDomains?: string[]
  excludeDomains?: string[]
  startPublishedDate?: string
  category?: string
}

export interface ExaResult {
  title: string
  url: string
  text?: string
  score: number
  publishedDate?: string
}

export async function search(
  query: string,
  options: ExaSearchOptions = {}
): Promise<{ results: ExaResult[]; autoprompt?: string }> {
  const client = getClient()
  const result = await client.search(query, {
    type: options.searchType,
    numResults: options.numResults,
    includeDomains: options.includeDomains,
    excludeDomains: options.excludeDomains,
    startPublishedDate: options.startPublishedDate,
    category: options.category,
    contents: { text: true },
  }) as any
  return {
    results: result.results.map((r: any) => ({
      title: r.title,
      url: r.url,
      text: r.text,
      score: r.score,
      publishedDate: r.publishedDate,
    })),
    autoprompt: result.autopromptString,
  }
}
```

- [x] **Step 6: Run OpenRouter tests — expect PASS**

```bash
npm test __tests__/lib/openrouter.test.ts
```

Expected: 2 passing.

- [x] **Step 7: Run all tests**

```bash
npm test
```

Expected: all passing.

- [x] **Step 8: Commit**

```bash
git add src/lib/openrouter.ts src/lib/firecrawl.ts src/lib/exa.ts \
        __tests__/lib/openrouter.test.ts
git commit -m "feat: add OpenRouter, Firecrawl, Exa client libs"
```

---

### Task 7: Stripe lib

**Files:**
- Create: `src/lib/stripe.ts`
- Create: `__tests__/lib/stripe.test.ts`

- [x] **Step 1: Install Stripe**

Already installed in Task 1. If not: `npm install stripe`

- [x] **Step 2: Write failing tests**

Create `__tests__/lib/stripe.test.ts`:
```ts
import { describe, it, expect, beforeEach } from 'vitest'
import { getStripeClient } from '@/lib/stripe'

describe('stripe lib', () => {
  beforeEach(() => {
    process.env.STRIPE_SECRET_KEY = 'sk_test_fake'
  })

  it('getStripeClient returns a Stripe instance', () => {
    const stripe = getStripeClient()
    expect(stripe).toBeDefined()
    expect(typeof stripe.checkout.sessions.create).toBe('function')
  })
})
```

- [x] **Step 3: Create `src/lib/stripe.ts`**

```ts
import Stripe from 'stripe'

let stripeClient: Stripe | null = null

export function getStripeClient(): Stripe {
  if (!stripeClient) {
    stripeClient = new Stripe(process.env.STRIPE_SECRET_KEY!, {
      apiVersion: '2024-11-20.acacia',
    })
  }
  return stripeClient
}

export async function createCheckoutSession(
  type: 'subscription' | 'topup',
  userId: string,
  customerEmail?: string
): Promise<string> {
  const stripe = getStripeClient()
  const priceId = type === 'subscription'
    ? process.env.STRIPE_PRO_PRICE_ID!
    : process.env.STRIPE_TOPUP_PRICE_ID!
  const appUrl = process.env.NEXT_PUBLIC_APP_URL ?? 'https://api.shepherd.codes'

  const session = await stripe.checkout.sessions.create({
    mode: type === 'subscription' ? 'subscription' : 'payment',
    line_items: [{ price: priceId, quantity: 1 }],
    success_url: `${appUrl}/checkout/success?session_id={CHECKOUT_SESSION_ID}`,
    cancel_url: `${appUrl}/checkout/cancel`,
    customer_email: customerEmail,
    metadata: { user_id: userId },
  })
  return session.url!
}

export function verifyWebhookSignature(payload: string, sig: string): Stripe.Event {
  return getStripeClient().webhooks.constructEvent(
    payload,
    sig,
    process.env.STRIPE_WEBHOOK_SECRET!
  )
}
```

- [x] **Step 4: Run tests — expect PASS**

```bash
npm test __tests__/lib/stripe.test.ts
```

- [x] **Step 5: Run all tests**

```bash
npm test
```

Expected: all passing.

- [x] **Step 6: Commit**

```bash
git add src/lib/stripe.ts __tests__/lib/stripe.test.ts
git commit -m "feat: add Stripe client lib — checkout session + webhook verification"
```

---

## Chunk 3: Auth + Credits Routes

### Task 8: Auth routes

**Files:**
- Create: `src/app/api/auth/login/route.ts`
- Create: `src/app/api/auth/callback/route.ts`
- Create: `__tests__/routes/auth.test.ts`

- [x] **Step 1: Write failing tests**

Create `__tests__/routes/auth.test.ts`:
```ts
import { describe, it, expect } from 'vitest'
import { GET as loginGET } from '@/app/api/auth/login/route'
import { GET as callbackGET } from '@/app/api/auth/callback/route'
import { NextRequest } from 'next/server'

describe('GET /api/auth/login', () => {
  it('redirects to Supabase OAuth when provider=github', async () => {
    process.env.SUPABASE_URL = 'https://test.supabase.co'
    process.env.NEXT_PUBLIC_APP_URL = 'https://api.shepherd.codes'
    const req = new NextRequest(
      'http://localhost/api/auth/login?provider=github'
    )
    const res = await loginGET(req)
    expect(res.status).toBe(302)
    expect(res.headers.get('location')).toContain('supabase.co')
  })
})

describe('GET /api/auth/callback', () => {
  it('returns 400 when no code provided', async () => {
    const req = new NextRequest('http://localhost/api/auth/callback')
    const res = await callbackGET(req)
    expect(res.status).toBe(400)
  })
})
```

- [x] **Step 2: Run tests — expect FAIL**

```bash
npm test __tests__/routes/auth.test.ts
```

- [x] **Step 3: Create `src/app/api/auth/login/route.ts`**

```ts
import { NextRequest, NextResponse } from 'next/server'
import { getAnonClient } from '@/lib/supabase'

export async function GET(req: NextRequest) {
  const url = new URL(req.url)
  const provider = url.searchParams.get('provider')
  const email = url.searchParams.get('email')
  const appUrl = process.env.NEXT_PUBLIC_APP_URL ?? 'https://api.shepherd.codes'
  const redirectTo = `${appUrl}/api/auth/callback`

  const client = getAnonClient()

  if (provider === 'github') {
    const { data, error } = await client.auth.signInWithOAuth({
      provider: 'github',
      options: { redirectTo },
    })
    if (error || !data.url) {
      return NextResponse.json({ error: 'oauth_failed' }, { status: 500 })
    }
    return NextResponse.redirect(data.url)
  }

  if (email) {
    await client.auth.signInWithOtp({ email, options: { emailRedirectTo: redirectTo } })
    return NextResponse.json({ message: 'magic link sent' })
  }

  // Default: GitHub
  const { data, error } = await client.auth.signInWithOAuth({
    provider: 'github',
    options: { redirectTo },
  })
  if (error || !data.url) {
    return NextResponse.json({ error: 'oauth_failed' }, { status: 500 })
  }
  return NextResponse.redirect(data.url)
}
```

- [x] **Step 4: Create `src/app/api/auth/callback/route.ts`**

```ts
import { NextRequest, NextResponse } from 'next/server'
import { getAnonClient } from '@/lib/supabase'

export async function GET(req: NextRequest) {
  const url = new URL(req.url)
  const code = url.searchParams.get('code')

  if (!code) {
    return NextResponse.json({ error: 'missing_code' }, { status: 400 })
  }

  const client = getAnonClient()
  const { data, error } = await client.auth.exchangeCodeForSession(code)

  if (error || !data.session) {
    return NextResponse.json({ error: 'exchange_failed' }, { status: 500 })
  }

  const { access_token, refresh_token } = data.session
  const deepLink = `shepherd://auth/callback?access_token=${access_token}&refresh_token=${refresh_token}`
  return NextResponse.redirect(deepLink)
}
```

- [x] **Step 5: Run tests — expect PASS**

```bash
npm test __tests__/routes/auth.test.ts
```

Expected: 2 passing.

- [x] **Step 6: Commit**

```bash
git add src/app/api/auth/ __tests__/routes/auth.test.ts
git commit -m "feat: add auth/login and auth/callback routes"
```

---

### Task 9: Credits routes — balance + purchase

**Files:**
- Create: `src/app/api/credits/balance/route.ts`
- Create: `src/app/api/credits/purchase/route.ts`
- Create: `__tests__/routes/credits.test.ts`

- [x] **Step 1: Write failing tests**

Create `__tests__/routes/credits.test.ts`:
```ts
import { describe, it, expect, vi } from 'vitest'
import { server } from '../setup'
import { http, HttpResponse } from 'msw'
import { NextRequest } from 'next/server'

vi.mock('@/lib/supabase', () => ({
  verifyJwt: vi.fn(),
  getAdminClient: vi.fn(),
}))
vi.mock('@/lib/stripe', () => ({
  createCheckoutSession: vi.fn(),
}))

import { GET as balanceGET } from '@/app/api/credits/balance/route'
import { POST as purchasePOST } from '@/app/api/credits/purchase/route'
import { verifyJwt, getAdminClient } from '@/lib/supabase'
import { createCheckoutSession } from '@/lib/stripe'

const SUPABASE_URL = 'https://test.supabase.co'

function makeReq(path: string, auth?: string) {
  return new NextRequest(`http://localhost${path}`, {
    headers: auth ? { authorization: auth } : {},
  })
}

describe('GET /api/credits/balance', () => {
  it('returns 401 when not authenticated', async () => {
    ;(verifyJwt as any).mockResolvedValue(null)
    const res = await balanceGET(makeReq('/api/credits/balance'))
    expect(res.status).toBe(401)
  })

  it('returns balance JSON when authenticated', async () => {
    ;(verifyJwt as any).mockResolvedValue({ id: 'u1', email: 'a@b.com' })
    // Mock the Supabase query
    const mockRpc = vi.fn().mockResolvedValue({
      data: [{
        plan: 'pro', credits_balance: 42,
        email: 'a@b.com', github_handle: 'gh',
        trial_logo: 2, trial_name: 1, trial_northstar: 2,
        trial_scrape: 2, trial_crawl: 2, trial_vision: 2, trial_search: 2,
      }],
      error: null,
    })
    ;(getAdminClient as any).mockReturnValue({ rpc: mockRpc })
    const res = await balanceGET(makeReq('/api/credits/balance', 'Bearer token'))
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.credits_balance).toBe(42)
    expect(body.plan).toBe('pro')
  })
})

describe('POST /api/credits/purchase', () => {
  it('returns 401 when not authenticated', async () => {
    ;(verifyJwt as any).mockResolvedValue(null)
    const res = await purchasePOST(makeReq('/api/credits/purchase'))
    expect(res.status).toBe(401)
  })

  it('redirects to Stripe checkout when authenticated', async () => {
    ;(verifyJwt as any).mockResolvedValue({ id: 'u1', email: 'a@b.com' })
    ;(createCheckoutSession as any).mockResolvedValue('https://checkout.stripe.com/test')
    const req = new NextRequest('http://localhost/api/credits/purchase', {
      method: 'POST',
      headers: { authorization: 'Bearer token', 'content-type': 'application/json' },
      body: JSON.stringify({ type: 'subscription' }),
    })
    const res = await purchasePOST(req)
    expect(res.status).toBe(302)
    expect(res.headers.get('location')).toContain('stripe.com')
  })
})
```

- [x] **Step 2: Run tests — expect FAIL**

```bash
npm test __tests__/routes/credits.test.ts
```

- [x] **Step 3: Create `src/app/api/credits/balance/route.ts`**

```ts
import { NextRequest, NextResponse } from 'next/server'
import { authenticate } from '@/lib/middleware'
import { getAdminClient } from '@/lib/supabase'

export async function GET(req: NextRequest) {
  const auth = await authenticate(req)
  if (auth instanceof NextResponse) return auth

  const db = getAdminClient()
  const { data, error } = await db.rpc('get_balance_with_trials', {
    p_user_id: auth.user.id,
  })

  if (error || !data || data.length === 0) {
    return NextResponse.json({ error: 'profile_not_found' }, { status: 404 })
  }

  const row = data[0]
  return NextResponse.json({
    plan: row.plan,
    credits_balance: row.credits_balance,
    email: row.email,
    github_handle: row.github_handle,
    trial_logo:      row.trial_logo,
    trial_name:      row.trial_name,
    trial_northstar: row.trial_northstar,
    trial_scrape:    row.trial_scrape,
    trial_crawl:     row.trial_crawl,
    trial_vision:    row.trial_vision,
    trial_search:    row.trial_search,
  })
}
```

Add this SQL function to a new migration file `supabase/migrations/002_balance_fn.sql`:
```sql
create or replace function public.get_balance_with_trials(p_user_id uuid)
returns table (
  plan text, credits_balance int, email text, github_handle text,
  trial_logo int, trial_name int, trial_northstar int,
  trial_scrape int, trial_crawl int, trial_vision int, trial_search int
) language sql security definer as $$
  select
    p.plan, p.credits_balance, p.email, p.github_handle,
    coalesce((select uses_remaining from trial_usage where user_id = p.id and feature = 'logo'),      2),
    coalesce((select uses_remaining from trial_usage where user_id = p.id and feature = 'name'),      2),
    coalesce((select uses_remaining from trial_usage where user_id = p.id and feature = 'northstar'), 2),
    coalesce((select uses_remaining from trial_usage where user_id = p.id and feature = 'scrape'),    2),
    coalesce((select uses_remaining from trial_usage where user_id = p.id and feature = 'crawl'),     2),
    coalesce((select uses_remaining from trial_usage where user_id = p.id and feature = 'vision'),    2),
    coalesce((select uses_remaining from trial_usage where user_id = p.id and feature = 'search'),    2)
  from profiles p where p.id = p_user_id;
$$;
```

- [x] **Step 4: Create `src/app/api/credits/purchase/route.ts`**

```ts
import { NextRequest, NextResponse } from 'next/server'
import { authenticate } from '@/lib/middleware'
import { createCheckoutSession } from '@/lib/stripe'

export async function POST(req: NextRequest) {
  const auth = await authenticate(req)
  if (auth instanceof NextResponse) return auth

  const body = await req.json().catch(() => ({}))
  const type = body.type === 'topup' ? 'topup' : 'subscription'

  const url = await createCheckoutSession(type, auth.user.id, auth.user.email)
  return NextResponse.redirect(url)
}
```

- [x] **Step 5: Run tests — expect PASS**

```bash
npm test __tests__/routes/credits.test.ts
```

- [x] **Step 6: Commit**

```bash
git add src/app/api/credits/ supabase/migrations/002_balance_fn.sql \
        __tests__/routes/credits.test.ts
git commit -m "feat: add credits/balance and credits/purchase routes"
```

---

## Chunk 4: Generation Routes — Logo, Name, North Star

### Task 10: Generation helper — shared route factory

**Files:**
- Create: `src/lib/generation.ts`

A shared factory reduces boilerplate across 7 identical middleware pipelines.

- [x] **Step 1: Create `src/lib/generation.ts`**

```ts
import { NextRequest, NextResponse } from 'next/server'
import { authenticate, rateLimit, errorResponse } from './middleware'
import { checkTrial, spendTrial, deductCredits, saveGeneration } from './credits'

export const CREDIT_COSTS: Record<string, number> = {
  logo: 2, name: 1, northstar: 15,
  scrape: 1, crawl: 5, vision: 2, search: 1,
}

export async function generationHandler<T>(
  req: NextRequest,
  feature: string,
  handler: (userId: string, body: T) => Promise<{ result: object; response: object }>
): Promise<NextResponse> {
  // 1. Auth
  const auth = await authenticate(req)
  if (auth instanceof NextResponse) return auth
  const { user } = auth

  // 2. Rate limit
  const limited = await rateLimit(user.id, feature)
  if (limited) return limited

  // 3. Parse body
  let body: T
  try {
    body = await req.json()
  } catch {
    return errorResponse(400, 'invalid_json')
  }

  // 4. Trial or credits
  const creditCost = CREDIT_COSTS[feature] ?? 1
  const trialsLeft = await checkTrial(user.id, feature)

  let creditsRemaining: number
  let usedTrial = false

  if (trialsLeft > 0) {
    usedTrial = true
    creditsRemaining = trialsLeft - 1 // display approximate
  } else {
    try {
      creditsRemaining = await deductCredits(user.id, creditCost, feature)
    } catch (err: any) {
      if (err.message?.includes('insufficient_credits')) {
        return NextResponse.json(
          { error: 'credits_insufficient', need: creditCost, have: 0,
            upgrade_url: `${process.env.NEXT_PUBLIC_APP_URL}/api/credits/purchase` },
          { status: 402 }
        )
      }
      return errorResponse(500, 'upstream_error', { message: err.message })
    }
  }

  // 5. Call upstream
  let result: object
  let responsePayload: object
  try {
    const out = await handler(user.id, body)
    result = out.result
    responsePayload = out.response
  } catch (err: any) {
    // Refund trial spend on upstream failure
    if (usedTrial) {
      // trial not yet decremented — nothing to undo
    }
    return errorResponse(502, 'upstream_error', { message: err.message })
  }

  // 6. Spend trial (after successful upstream call)
  if (usedTrial) await spendTrial(user.id, feature)

  // 7. Save generation
  const generationId = await saveGeneration(user.id, feature, usedTrial ? 0 : creditCost, body as object, result)

  return NextResponse.json({ generation_id: generationId, ...responsePayload, credits_remaining: creditsRemaining })
}
```

- [x] **Step 2: Commit**

```bash
git add src/lib/generation.ts
git commit -m "feat: add generationHandler factory — shared middleware pipeline"
```

---

### Task 11: Logo generation route

**Files:**
- Create: `src/app/api/generate/logo/route.ts`
- Create: `__tests__/routes/generate/logo.test.ts`

- [x] **Step 1: Write failing tests**

Create `__tests__/routes/generate/logo.test.ts`:
```ts
import { describe, it, expect, vi } from 'vitest'
import { server } from '../../setup'
import { http, HttpResponse } from 'msw'
import { NextRequest } from 'next/server'

vi.mock('@/lib/supabase', () => ({ verifyJwt: vi.fn(), getAdminClient: vi.fn() }))
vi.mock('@/lib/credits', () => ({
  checkTrial: vi.fn().mockResolvedValue(0),
  spendTrial: vi.fn(),
  deductCredits: vi.fn().mockResolvedValue(40),
  saveGeneration: vi.fn().mockResolvedValue('gen-uuid'),
}))

import { POST } from '@/app/api/generate/logo/route'
import { verifyJwt } from '@/lib/supabase'

function makeReq(body: object, auth = true) {
  return new NextRequest('http://localhost/api/generate/logo', {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      ...(auth ? { authorization: 'Bearer token' } : {}),
    },
    body: JSON.stringify(body),
  })
}

const OR_BASE = 'https://openrouter.ai/api/v1'

describe('POST /api/generate/logo', () => {
  it('returns 401 when not authenticated', async () => {
    ;(verifyJwt as any).mockResolvedValue(null)
    const res = await POST(makeReq({}, false))
    expect(res.status).toBe(401)
  })

  it('returns variants on success', async () => {
    ;(verifyJwt as any).mockResolvedValue({ id: 'u1', email: 'a@b.com' })
    server.use(
      http.post(`${OR_BASE}/images/generations`, () =>
        HttpResponse.json({
          data: [
            { url: 'https://cdn.example.com/logo-0.png' },
            { url: 'https://cdn.example.com/logo-1.png' },
          ]
        })
      )
    )
    const res = await POST(makeReq({
      product_name: 'Shepherd',
      style: 'minimal',
      colors: ['#000'],
      variants: 2,
    }))
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.generation_id).toBe('gen-uuid')
    expect(body.variants).toHaveLength(2)
    expect(body.variants[0].url).toContain('logo-0')
    expect(body.credits_remaining).toBe(40)
  })

  it('returns 502 on OpenRouter error', async () => {
    ;(verifyJwt as any).mockResolvedValue({ id: 'u1', email: 'a@b.com' })
    server.use(
      http.post(`${OR_BASE}/images/generations`, () =>
        HttpResponse.json({ error: 'upstream error' }, { status: 500 })
      )
    )
    const res = await POST(makeReq({ product_name: 'X', style: 'minimal', variants: 1 }))
    expect(res.status).toBe(502)
  })
})
```

- [x] **Step 2: Run tests — expect FAIL**

```bash
npm test __tests__/routes/generate/logo.test.ts
```

- [x] **Step 3: Create `src/app/api/generate/logo/route.ts`**

```ts
import { NextRequest } from 'next/server'
import { generationHandler } from '@/lib/generation'
import { imageGeneration } from '@/lib/openrouter'

const MODEL = 'ideogram/ideogram-v2'

export async function POST(req: NextRequest) {
  return generationHandler(req, 'logo', async (_userId, body: any) => {
    const { product_name, product_description, style, colors = [], variants = 4 } = body
    const prompt = [
      `Professional logo for "${product_name}".`,
      product_description ? `Product: ${product_description}.` : '',
      `Style: ${style ?? 'minimal'}.`,
      colors.length ? `Colors: ${colors.join(', ')}.` : '',
      'Clean background, scalable, high contrast, suitable for app icon and marketing.',
    ].filter(Boolean).join(' ')

    const urls = await imageGeneration(MODEL, prompt, variants)
    const variantList = urls.map((url, index) => ({ index, url }))

    return {
      result: { variants: variantList },
      response: { variants: variantList },
    }
  })
}
```

- [x] **Step 4: Run tests — expect PASS**

```bash
npm test __tests__/routes/generate/logo.test.ts
```

- [x] **Step 5: Commit**

```bash
git add src/app/api/generate/logo/ __tests__/routes/generate/logo.test.ts
git commit -m "feat: add generate/logo route — Ideogram via OpenRouter"
```

---

### Task 12: Name generation route

**Files:**
- Create: `src/app/api/generate/name/route.ts`
- Create: `__tests__/routes/generate/name.test.ts`

- [x] **Step 1: Write failing tests**

Create `__tests__/routes/generate/name.test.ts`:
```ts
import { describe, it, expect, vi } from 'vitest'
import { server } from '../../setup'
import { http, HttpResponse } from 'msw'
import { NextRequest } from 'next/server'

vi.mock('@/lib/supabase', () => ({ verifyJwt: vi.fn(), getAdminClient: vi.fn() }))
vi.mock('@/lib/credits', () => ({
  checkTrial: vi.fn().mockResolvedValue(0),
  spendTrial: vi.fn(),
  deductCredits: vi.fn().mockResolvedValue(41),
  saveGeneration: vi.fn().mockResolvedValue('gen-uuid'),
}))

import { POST } from '@/app/api/generate/name/route'
import { verifyJwt } from '@/lib/supabase'

const OR_BASE = 'https://openrouter.ai/api/v1'

function makeReq(body: object, auth = true) {
  return new NextRequest('http://localhost/api/generate/name', {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      ...(auth ? { authorization: 'Bearer token' } : {}),
    },
    body: JSON.stringify(body),
  })
}

describe('POST /api/generate/name', () => {
  it('returns 401 when unauthenticated', async () => {
    ;(verifyJwt as any).mockResolvedValue(null)
    const res = await POST(makeReq({}, false))
    expect(res.status).toBe(401)
  })

  it('returns candidates on success', async () => {
    ;(verifyJwt as any).mockResolvedValue({ id: 'u1', email: 'a@b.com' })
    server.use(
      http.post(`${OR_BASE}/chat/completions`, () =>
        HttpResponse.json({
          choices: [{
            message: {
              content: JSON.stringify([
                { name: 'Shepherd', tagline: 'Herd your agents', reasoning: 'Clear metaphor', domains: [] }
              ])
            }
          }]
        })
      )
    )
    const res = await POST(makeReq({ description: 'AI agent manager', vibes: ['bold'] }))
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.generation_id).toBe('gen-uuid')
    expect(body.candidates[0].name).toBe('Shepherd')
  })
})
```

- [x] **Step 2: Run tests — expect FAIL**

```bash
npm test __tests__/routes/generate/name.test.ts
```

- [x] **Step 3: Create `src/app/api/generate/name/route.ts`**

```ts
import { NextRequest } from 'next/server'
import { generationHandler } from '@/lib/generation'
import { chatCompletion } from '@/lib/openrouter'

const MODEL = 'anthropic/claude-sonnet-4-6'

const SYSTEM_PROMPT = `You generate product name candidates. Return a JSON array of objects.
Each object: { name, tagline, reasoning, domains: [{ domain, available }] }.
Generate 20 candidates unless count is specified. Make names catchy, memorable, URL-friendly.
Check obvious domain availability heuristics (.com, .io, .codes).
Respond with ONLY the JSON array, no markdown.`

export async function POST(req: NextRequest) {
  return generationHandler(req, 'name', async (_userId, body: any) => {
    const { description, vibes = [], count = 20 } = body
    const userMsg = `Product: ${description}. Vibes: ${vibes.join(', ') || 'none'}. Generate ${count} names.`

    const raw = await chatCompletion(MODEL, [
      { role: 'system', content: SYSTEM_PROMPT },
      { role: 'user', content: userMsg },
    ], 0.9)

    let candidates: object[]
    try {
      candidates = JSON.parse(raw)
    } catch {
      // Attempt to extract JSON array from response
      const match = raw.match(/\[[\s\S]*\]/)
      candidates = match ? JSON.parse(match[0]) : []
    }

    return {
      result: { candidates },
      response: { candidates },
    }
  })
}
```

- [x] **Step 4: Run tests — expect PASS**

```bash
npm test __tests__/routes/generate/name.test.ts
```

- [x] **Step 5: Commit**

```bash
git add src/app/api/generate/name/ __tests__/routes/generate/name.test.ts
git commit -m "feat: add generate/name route — claude-sonnet-4-6 via OpenRouter"
```

---

### Task 13: North Star route

**Files:**
- Create: `src/app/api/generate/northstar/route.ts`
- Create: `__tests__/routes/generate/northstar.test.ts`

- [x] **Step 1: Write failing tests**

Create `__tests__/routes/generate/northstar.test.ts`:
```ts
import { describe, it, expect, vi } from 'vitest'
import { server } from '../../setup'
import { http, HttpResponse } from 'msw'
import { NextRequest } from 'next/server'

vi.mock('@/lib/supabase', () => ({ verifyJwt: vi.fn(), getAdminClient: vi.fn() }))
vi.mock('@/lib/credits', () => ({
  checkTrial: vi.fn().mockResolvedValue(0),
  spendTrial: vi.fn(),
  deductCredits: vi.fn().mockResolvedValue(27),
  saveGeneration: vi.fn().mockResolvedValue('gen-uuid'),
}))

import { POST } from '@/app/api/generate/northstar/route'
import { verifyJwt } from '@/lib/supabase'

const OR_BASE = 'https://openrouter.ai/api/v1'

function makeReq(body: object, auth = true) {
  return new NextRequest('http://localhost/api/generate/northstar', {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      ...(auth ? { authorization: 'Bearer token' } : {}),
    },
    body: JSON.stringify(body),
  })
}

describe('POST /api/generate/northstar', () => {
  it('returns 401 when unauthenticated', async () => {
    ;(verifyJwt as any).mockResolvedValue(null)
    const res = await POST(makeReq({}, false))
    expect(res.status).toBe(401)
  })

  it('returns phase result on success', async () => {
    ;(verifyJwt as any).mockResolvedValue({ id: 'u1', email: 'a@b.com' })
    server.use(
      http.post(`${OR_BASE}/chat/completions`, () =>
        HttpResponse.json({
          choices: [{ message: { content: '# Brand Strategy\n\nShepherd is...' } }]
        })
      )
    )
    const res = await POST(makeReq({ phase: 'brand', context: { name: 'Shepherd' } }))
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.generation_id).toBe('gen-uuid')
    expect(body.phase).toBe('brand')
    expect(body.result).toBeDefined()
  })

  it('returns 402 when insufficient credits', async () => {
    ;(verifyJwt as any).mockResolvedValue({ id: 'u1', email: 'a@b.com' })
    const { deductCredits } = await import('@/lib/credits')
    ;(deductCredits as any).mockRejectedValueOnce(new Error('insufficient_credits: need 15, have 0'))
    const res = await POST(makeReq({ phase: 'brand', context: {} }))
    expect(res.status).toBe(402)
  })
})
```

- [x] **Step 2: Run tests — expect FAIL**

```bash
npm test __tests__/routes/generate/northstar.test.ts
```

- [x] **Step 3: Create `src/app/api/generate/northstar/route.ts`**

```ts
import { NextRequest } from 'next/server'
import { generationHandler } from '@/lib/generation'
import { chatCompletion } from '@/lib/openrouter'

const MODEL = 'anthropic/claude-opus-4-6'

const PHASE_PROMPTS: Record<string, string> = {
  brand: 'Generate a comprehensive brand strategy document including mission, vision, values, and brand voice.',
  positioning: 'Analyse market positioning, identify differentiation, and write a positioning statement.',
  audience: 'Define target audience personas with demographics, psychographics, and jobs-to-be-done.',
  competitors: 'Research the competitive landscape and provide a competitive analysis.',
  messaging: 'Craft core messaging pillars, taglines, and value propositions.',
  gtm: 'Design a go-to-market strategy with channels, tactics, and milestones.',
}

export async function POST(req: NextRequest) {
  return generationHandler(req, 'northstar', async (_userId, body: any) => {
    const { phase, context = {} } = body
    const phasePrompt = PHASE_PROMPTS[phase] ?? `Generate strategic content for phase: ${phase}.`

    const result = await chatCompletion(MODEL, [
      {
        role: 'system',
        content: 'You are a world-class product strategist. Produce detailed, actionable strategic documents in Markdown.',
      },
      {
        role: 'user',
        content: `Context: ${JSON.stringify(context)}\n\nTask: ${phasePrompt}`,
      },
    ], 0.7)

    return {
      result: { content: result },
      response: { phase, result: { content: result } },
    }
  })
}
```

- [x] **Step 4: Run tests — expect PASS**

```bash
npm test __tests__/routes/generate/northstar.test.ts
```

- [x] **Step 5: Run all tests**

```bash
npm test
```

Expected: all passing.

- [x] **Step 6: Commit**

```bash
git add src/app/api/generate/northstar/ __tests__/routes/generate/northstar.test.ts
git commit -m "feat: add generate/northstar route — claude-opus-4-6 via OpenRouter"
```

---

## Chunk 5: Generation Routes — Scrape, Crawl, Vision, Search

### Task 14: Scrape route

**Files:**
- Create: `src/app/api/generate/scrape/route.ts`
- Create: `__tests__/routes/generate/scrape.test.ts`

- [x] **Step 1: Write failing tests**

Create `__tests__/routes/generate/scrape.test.ts`:
```ts
import { describe, it, expect, vi } from 'vitest'
import { NextRequest } from 'next/server'

vi.mock('@/lib/supabase', () => ({ verifyJwt: vi.fn(), getAdminClient: vi.fn() }))
vi.mock('@/lib/credits', () => ({
  checkTrial: vi.fn().mockResolvedValue(0),
  spendTrial: vi.fn(),
  deductCredits: vi.fn().mockResolvedValue(41),
  saveGeneration: vi.fn().mockResolvedValue('gen-uuid'),
}))
vi.mock('@/lib/firecrawl', () => ({ scrapePage: vi.fn() }))

import { POST } from '@/app/api/generate/scrape/route'
import { verifyJwt } from '@/lib/supabase'
import { scrapePage } from '@/lib/firecrawl'

function makeReq(body: object, auth = true) {
  return new NextRequest('http://localhost/api/generate/scrape', {
    method: 'POST',
    headers: { 'content-type': 'application/json', ...(auth ? { authorization: 'Bearer t' } : {}) },
    body: JSON.stringify(body),
  })
}

describe('POST /api/generate/scrape', () => {
  it('returns 401 when unauthenticated', async () => {
    ;(verifyJwt as any).mockResolvedValue(null)
    const res = await POST(makeReq({}, false))
    expect(res.status).toBe(401)
  })

  it('returns markdown on success', async () => {
    ;(verifyJwt as any).mockResolvedValue({ id: 'u1', email: 'a@b.com' })
    ;(scrapePage as any).mockResolvedValue({ markdown: '# Hello', links: [], metadata: {} })
    const res = await POST(makeReq({ url: 'https://example.com' }))
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.generation_id).toBe('gen-uuid')
    expect(body.markdown).toBe('# Hello')
  })

  it('returns 502 on Firecrawl error', async () => {
    ;(verifyJwt as any).mockResolvedValue({ id: 'u1', email: 'a@b.com' })
    ;(scrapePage as any).mockRejectedValue(new Error('Firecrawl error'))
    const res = await POST(makeReq({ url: 'https://example.com' }))
    expect(res.status).toBe(502)
  })
})
```

- [x] **Step 2: Run tests — expect FAIL**

```bash
npm test __tests__/routes/generate/scrape.test.ts
```

- [x] **Step 3: Create `src/app/api/generate/scrape/route.ts`**

```ts
import { NextRequest } from 'next/server'
import { generationHandler } from '@/lib/generation'
import { scrapePage } from '@/lib/firecrawl'

export async function POST(req: NextRequest) {
  return generationHandler(req, 'scrape', async (_userId, body: any) => {
    const { url, formats } = body
    const result = await scrapePage(url, formats)
    return {
      result,
      response: { markdown: result.markdown, links: result.links, metadata: result.metadata },
    }
  })
}
```

- [x] **Step 4: Run tests — expect PASS**

```bash
npm test __tests__/routes/generate/scrape.test.ts
```

- [x] **Step 5: Commit**

```bash
git add src/app/api/generate/scrape/ __tests__/routes/generate/scrape.test.ts
git commit -m "feat: add generate/scrape route — Firecrawl"
```

---

### Task 15: Crawl routes (start + poll)

**Files:**
- Create: `src/app/api/generate/crawl/route.ts`
- Create: `src/app/api/generate/crawl/[id]/route.ts`
- Create: `__tests__/routes/generate/crawl.test.ts`
- Create: `__tests__/routes/generate/crawl-status.test.ts`

- [x] **Step 1: Write failing tests for crawl start**

Create `__tests__/routes/generate/crawl.test.ts`:
```ts
import { describe, it, expect, vi } from 'vitest'
import { NextRequest } from 'next/server'

vi.mock('@/lib/supabase', () => ({ verifyJwt: vi.fn(), getAdminClient: vi.fn() }))
vi.mock('@/lib/credits', () => ({
  checkTrial: vi.fn().mockResolvedValue(0),
  spendTrial: vi.fn(),
  deductCredits: vi.fn().mockResolvedValue(37),
  saveGeneration: vi.fn().mockResolvedValue('gen-uuid'),
}))
vi.mock('@/lib/firecrawl', () => ({ startCrawl: vi.fn() }))

import { POST } from '@/app/api/generate/crawl/route'
import { verifyJwt } from '@/lib/supabase'
import { startCrawl } from '@/lib/firecrawl'

function makeReq(body: object, auth = true) {
  return new NextRequest('http://localhost/api/generate/crawl', {
    method: 'POST',
    headers: { 'content-type': 'application/json', ...(auth ? { authorization: 'Bearer t' } : {}) },
    body: JSON.stringify(body),
  })
}

describe('POST /api/generate/crawl', () => {
  it('returns 401 when unauthenticated', async () => {
    ;(verifyJwt as any).mockResolvedValue(null)
    const res = await POST(makeReq({}, false))
    expect(res.status).toBe(401)
  })

  it('returns crawl_id and status_url on success', async () => {
    ;(verifyJwt as any).mockResolvedValue({ id: 'u1', email: 'a@b.com' })
    ;(startCrawl as any).mockResolvedValue({ crawlId: 'fc-123' })
    process.env.NEXT_PUBLIC_APP_URL = 'https://api.shepherd.codes'
    const res = await POST(makeReq({ url: 'https://example.com' }))
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.crawl_id).toBe('fc-123')
    expect(body.status_url).toBe('https://api.shepherd.codes/api/generate/crawl/fc-123')
  })
})
```

- [x] **Step 2: Write failing tests for poll**

Create `__tests__/routes/generate/crawl-status.test.ts`:
```ts
import { describe, it, expect, vi } from 'vitest'
import { NextRequest } from 'next/server'

vi.mock('@/lib/supabase', () => ({ verifyJwt: vi.fn() }))
vi.mock('@/lib/firecrawl', () => ({ getCrawlStatus: vi.fn() }))

import { GET } from '@/app/api/generate/crawl/[id]/route'
import { verifyJwt } from '@/lib/supabase'
import { getCrawlStatus } from '@/lib/firecrawl'

describe('GET /api/generate/crawl/[id]', () => {
  it('returns 401 when unauthenticated', async () => {
    ;(verifyJwt as any).mockResolvedValue(null)
    const req = new NextRequest('http://localhost/api/generate/crawl/abc')
    const res = await GET(req, { params: Promise.resolve({ id: 'abc' }) })
    expect(res.status).toBe(401)
  })

  it('returns crawl status when authenticated', async () => {
    ;(verifyJwt as any).mockResolvedValue({ id: 'u1', email: 'a@b.com' })
    ;(getCrawlStatus as any).mockResolvedValue({
      success: true, status: 'completed', total: 5, completed: 5, data: [],
    })
    const req = new NextRequest('http://localhost/api/generate/crawl/fc-123', {
      headers: { authorization: 'Bearer token' },
    })
    const res = await GET(req, { params: Promise.resolve({ id: 'fc-123' }) })
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.status).toBe('completed')
    expect(body.total).toBe(5)
  })
})
```

- [x] **Step 3: Run tests — expect FAIL**

```bash
npm test __tests__/routes/generate/crawl.test.ts __tests__/routes/generate/crawl-status.test.ts
```

- [x] **Step 4: Create `src/app/api/generate/crawl/route.ts`**

```ts
import { NextRequest } from 'next/server'
import { generationHandler } from '@/lib/generation'
import { startCrawl } from '@/lib/firecrawl'

export async function POST(req: NextRequest) {
  return generationHandler(req, 'crawl', async (_userId, body: any) => {
    const { url, max_depth, limit } = body
    const { crawlId } = await startCrawl(url, max_depth, limit)
    const appUrl = process.env.NEXT_PUBLIC_APP_URL ?? 'https://api.shepherd.codes'
    const statusUrl = `${appUrl}/api/generate/crawl/${crawlId}`

    return {
      result: { crawl_id: crawlId },
      response: { crawl_id: crawlId, status_url: statusUrl },
    }
  })
}
```

- [x] **Step 5: Create `src/app/api/generate/crawl/[id]/route.ts`**

```ts
import { NextRequest, NextResponse } from 'next/server'
import { authenticate } from '@/lib/middleware'
import { getCrawlStatus } from '@/lib/firecrawl'

export async function GET(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> }
) {
  const auth = await authenticate(req)
  if (auth instanceof NextResponse) return auth

  const { id } = await params
  try {
    const status = await getCrawlStatus(id)
    return NextResponse.json(status)
  } catch (err: any) {
    return NextResponse.json({ error: 'upstream_error', message: err.message }, { status: 502 })
  }
}
```

- [x] **Step 6: Run tests — expect PASS**

```bash
npm test __tests__/routes/generate/crawl.test.ts __tests__/routes/generate/crawl-status.test.ts
```

- [x] **Step 7: Commit**

```bash
git add src/app/api/generate/crawl/ \
        __tests__/routes/generate/crawl.test.ts \
        __tests__/routes/generate/crawl-status.test.ts
git commit -m "feat: add generate/crawl routes — start + poll — Firecrawl"
```

---

### Task 16: Vision route

**Files:**
- Create: `src/app/api/generate/vision/route.ts`
- Create: `__tests__/routes/generate/vision.test.ts`

- [x] **Step 1: Write failing tests**

Create `__tests__/routes/generate/vision.test.ts`:
```ts
import { describe, it, expect, vi } from 'vitest'
import { server } from '../../setup'
import { http, HttpResponse } from 'msw'
import { NextRequest } from 'next/server'

vi.mock('@/lib/supabase', () => ({ verifyJwt: vi.fn(), getAdminClient: vi.fn() }))
vi.mock('@/lib/credits', () => ({
  checkTrial: vi.fn().mockResolvedValue(0),
  spendTrial: vi.fn(),
  deductCredits: vi.fn().mockResolvedValue(40),
  saveGeneration: vi.fn().mockResolvedValue('gen-uuid'),
}))

import { POST } from '@/app/api/generate/vision/route'
import { verifyJwt } from '@/lib/supabase'

const OR_BASE = 'https://openrouter.ai/api/v1'

function makeReq(body: object, auth = true) {
  return new NextRequest('http://localhost/api/generate/vision', {
    method: 'POST',
    headers: { 'content-type': 'application/json', ...(auth ? { authorization: 'Bearer t' } : {}) },
    body: JSON.stringify(body),
  })
}

describe('POST /api/generate/vision', () => {
  it('returns 401 when unauthenticated', async () => {
    ;(verifyJwt as any).mockResolvedValue(null)
    const res = await POST(makeReq({}, false))
    expect(res.status).toBe(401)
  })

  it('returns analysis on success with image_url', async () => {
    ;(verifyJwt as any).mockResolvedValue({ id: 'u1', email: 'a@b.com' })
    server.use(
      http.post(`${OR_BASE}/chat/completions`, () =>
        HttpResponse.json({ choices: [{ message: { content: 'This is a screenshot of...' } }] })
      )
    )
    const res = await POST(makeReq({
      image_url: 'https://example.com/screenshot.png',
      prompt: 'Describe this UI',
    }))
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.generation_id).toBe('gen-uuid')
    expect(body.analysis).toContain('screenshot')
  })
})
```

- [x] **Step 2: Run tests — expect FAIL**

```bash
npm test __tests__/routes/generate/vision.test.ts
```

- [x] **Step 3: Create `src/app/api/generate/vision/route.ts`**

```ts
import { NextRequest, NextResponse } from 'next/server'
import { generationHandler } from '@/lib/generation'
import { chatCompletion, visionMessage } from '@/lib/openrouter'

const MODEL = 'anthropic/claude-sonnet-4-6'

export async function POST(req: NextRequest) {
  return generationHandler(req, 'vision', async (_userId, body: any) => {
    const { image_url, image_base64, prompt } = body

    if (!image_url && !image_base64) {
      throw new Error('Either image_url or image_base64 is required')
    }

    const msg = visionMessage(prompt, image_url, image_base64)
    const analysis = await chatCompletion(MODEL, [msg], 0.3)

    return {
      result: { analysis },
      response: { analysis },
    }
  })
}
```

- [x] **Step 4: Run tests — expect PASS**

```bash
npm test __tests__/routes/generate/vision.test.ts
```

- [x] **Step 5: Commit**

```bash
git add src/app/api/generate/vision/ __tests__/routes/generate/vision.test.ts
git commit -m "feat: add generate/vision route — claude-sonnet-4-6 via OpenRouter"
```

---

### Task 17: Search route

**Files:**
- Create: `src/app/api/generate/search/route.ts`
- Create: `__tests__/routes/generate/search.test.ts`

- [x] **Step 1: Write failing tests**

Create `__tests__/routes/generate/search.test.ts`:
```ts
import { describe, it, expect, vi } from 'vitest'
import { NextRequest } from 'next/server'

vi.mock('@/lib/supabase', () => ({ verifyJwt: vi.fn(), getAdminClient: vi.fn() }))
vi.mock('@/lib/credits', () => ({
  checkTrial: vi.fn().mockResolvedValue(0),
  spendTrial: vi.fn(),
  deductCredits: vi.fn().mockResolvedValue(41),
  saveGeneration: vi.fn().mockResolvedValue('gen-uuid'),
}))
vi.mock('@/lib/exa', () => ({ search: vi.fn() }))

import { POST } from '@/app/api/generate/search/route'
import { verifyJwt } from '@/lib/supabase'
import { search } from '@/lib/exa'

function makeReq(body: object, auth = true) {
  return new NextRequest('http://localhost/api/generate/search', {
    method: 'POST',
    headers: { 'content-type': 'application/json', ...(auth ? { authorization: 'Bearer t' } : {}) },
    body: JSON.stringify(body),
  })
}

describe('POST /api/generate/search', () => {
  it('returns 401 when unauthenticated', async () => {
    ;(verifyJwt as any).mockResolvedValue(null)
    const res = await POST(makeReq({}, false))
    expect(res.status).toBe(401)
  })

  it('returns results on success', async () => {
    ;(verifyJwt as any).mockResolvedValue({ id: 'u1', email: 'a@b.com' })
    ;(search as any).mockResolvedValue({
      results: [{ title: 'Example', url: 'https://example.com', score: 0.9 }],
      autoprompt: 'best results for...',
    })
    const res = await POST(makeReq({ query: 'AI coding agents', num_results: 5 }))
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.generation_id).toBe('gen-uuid')
    expect(body.results[0].title).toBe('Example')
    expect(body.autoprompt).toBe('best results for...')
  })
})
```

- [x] **Step 2: Run tests — expect FAIL**

```bash
npm test __tests__/routes/generate/search.test.ts
```

- [x] **Step 3: Create `src/app/api/generate/search/route.ts`**

```ts
import { NextRequest } from 'next/server'
import { generationHandler } from '@/lib/generation'
import { search } from '@/lib/exa'

export async function POST(req: NextRequest) {
  return generationHandler(req, 'search', async (_userId, body: any) => {
    const {
      query, search_type, num_results,
      include_domains, exclude_domains,
      start_published_date, category,
    } = body

    const { results, autoprompt } = await search(query, {
      searchType: search_type,
      numResults: num_results,
      includeDomains: include_domains,
      excludeDomains: exclude_domains,
      startPublishedDate: start_published_date,
      category,
    })

    return {
      result: { results, autoprompt },
      response: { results, autoprompt },
    }
  })
}
```

- [x] **Step 4: Run tests — expect PASS**

```bash
npm test __tests__/routes/generate/search.test.ts
```

- [x] **Step 5: Run all tests**

```bash
npm test
```

Expected: all passing.

- [x] **Step 6: Commit**

```bash
git add src/app/api/generate/search/ __tests__/routes/generate/search.test.ts
git commit -m "feat: add generate/search route — Exa"
```

---

## Chunk 6: Stripe Webhook

### Task 18: Stripe webhook handler

**Files:**
- Create: `src/app/api/webhooks/stripe/route.ts`
- Create: `__tests__/routes/webhooks/stripe.test.ts`

- [x] **Step 1: Write failing tests**

Create `__tests__/routes/webhooks/stripe.test.ts`:
```ts
import { describe, it, expect, vi } from 'vitest'
import { NextRequest } from 'next/server'
import Stripe from 'stripe'

vi.mock('@/lib/stripe', () => ({ verifyWebhookSignature: vi.fn(), getStripeClient: vi.fn() }))
vi.mock('@/lib/supabase', () => ({ getAdminClient: vi.fn() }))

import { POST } from '@/app/api/webhooks/stripe/route'
import { verifyWebhookSignature } from '@/lib/stripe'
import { getAdminClient } from '@/lib/supabase'

function makeWebhookReq(event: object) {
  return new NextRequest('http://localhost/api/webhooks/stripe', {
    method: 'POST',
    headers: { 'stripe-signature': 'sig_test', 'content-type': 'application/json' },
    body: JSON.stringify(event),
  })
}

const mockDb = () => ({
  from: () => ({
    update: () => ({ eq: () => Promise.resolve({ error: null }) }),
    select: () => ({ eq: () => ({ single: () => Promise.resolve({ data: { id: 'u1', credits_balance: 10 }, error: null }) }) }),
    insert: () => Promise.resolve({ error: null }),
  }),
  rpc: vi.fn().mockResolvedValue({ data: 50, error: null }),
})

describe('POST /api/webhooks/stripe', () => {
  it('returns 400 when signature invalid', async () => {
    ;(verifyWebhookSignature as any).mockImplementation(() => { throw new Error('Invalid signature') })
    const res = await POST(makeWebhookReq({ type: 'checkout.session.completed' }))
    expect(res.status).toBe(400)
  })

  it('handles checkout.session.completed subscription', async () => {
    const event: Partial<Stripe.Event> = {
      type: 'checkout.session.completed',
      data: { object: { mode: 'subscription', customer: 'cus_1', subscription: 'sub_1', metadata: { user_id: 'u1' } } as any },
    }
    ;(verifyWebhookSignature as any).mockReturnValue(event)
    ;(getAdminClient as any).mockReturnValue(mockDb())
    const res = await POST(makeWebhookReq(event))
    expect(res.status).toBe(200)
  })

  it('handles customer.subscription.deleted', async () => {
    const event: Partial<Stripe.Event> = {
      type: 'customer.subscription.deleted',
      data: { object: { customer: 'cus_1' } as any },
    }
    ;(verifyWebhookSignature as any).mockReturnValue(event)
    ;(getAdminClient as any).mockReturnValue(mockDb())
    const res = await POST(makeWebhookReq(event))
    expect(res.status).toBe(200)
  })

  it('returns 200 for unhandled event types', async () => {
    const event = { type: 'payment_intent.created', data: { object: {} } }
    ;(verifyWebhookSignature as any).mockReturnValue(event)
    const res = await POST(makeWebhookReq(event))
    expect(res.status).toBe(200)
  })
})
```

- [x] **Step 2: Run tests — expect FAIL**

```bash
npm test __tests__/routes/webhooks/stripe.test.ts
```

- [x] **Step 3: Create `src/app/api/webhooks/stripe/route.ts`**

```ts
import { NextRequest, NextResponse } from 'next/server'
import { verifyWebhookSignature } from '@/lib/stripe'
import { getAdminClient } from '@/lib/supabase'

export async function POST(req: NextRequest) {
  const body = await req.text()
  const sig = req.headers.get('stripe-signature') ?? ''

  let event: ReturnType<typeof verifyWebhookSignature>
  try {
    event = verifyWebhookSignature(body, sig)
  } catch {
    return NextResponse.json({ error: 'invalid_signature' }, { status: 400 })
  }

  const db = getAdminClient()

  switch (event.type) {
    case 'checkout.session.completed': {
      const session = event.data.object as any
      const userId = session.metadata?.user_id
      if (!userId) break

      if (session.mode === 'subscription') {
        // Grant Pro plan
        await db.from('profiles').update({
          plan: 'pro',
          stripe_customer_id: session.customer,
          stripe_subscription_id: session.subscription,
          updated_at: new Date().toISOString(),
        }).eq('id', userId)
      } else if (session.mode === 'payment') {
        // Top-up: +30 credits
        await db.rpc('deduct_credits', {
          p_user_id: userId,
          p_amount: -30, // negative = credit grant
          p_description: 'topup',
        }).catch(() => {
          // fallback: direct update
          db.from('profiles').update({
            credits_balance: db.rpc as any, // handled below
          })
        })
        await db.from('credit_transactions').insert({
          user_id: userId,
          amount: 30,
          balance_after: 0, // approximate — balance route will show real value
          type: 'topup',
          description: 'Credit top-up purchase',
        })
      }
      break
    }

    case 'invoice.payment_succeeded': {
      const invoice = event.data.object as any
      const customerId = invoice.customer
      const { data: profile } = await db
        .from('profiles')
        .select('id')
        .eq('stripe_customer_id', customerId)
        .single()
      if (!profile) break

      // Reset monthly credits to 50
      await db.from('profiles').update({
        credits_balance: 50,
        updated_at: new Date().toISOString(),
      }).eq('id', profile.id)
      await db.from('credit_transactions').insert({
        user_id: profile.id,
        amount: 50,
        balance_after: 50,
        type: 'subscription_grant',
        description: 'Monthly credit reset',
      })
      break
    }

    case 'invoice.payment_failed':
      // No action — credits frozen until payment succeeds
      break

    case 'customer.subscription.updated': {
      const sub = event.data.object as any
      await db.from('profiles').update({
        stripe_subscription_id: sub.id,
        updated_at: new Date().toISOString(),
      }).eq('stripe_customer_id', sub.customer)
      break
    }

    case 'customer.subscription.deleted': {
      const sub = event.data.object as any
      await db.from('profiles').update({
        plan: 'free',
        credits_balance: 0,
        stripe_subscription_id: null,
        updated_at: new Date().toISOString(),
      }).eq('stripe_customer_id', sub.customer)
      break
    }

    default:
      // Unknown event — ignore
      break
  }

  return NextResponse.json({ received: true })
}
```

- [x] **Step 4: Run tests — expect PASS**

```bash
npm test __tests__/routes/webhooks/stripe.test.ts
```

- [x] **Step 5: Run all tests**

```bash
npm test
```

Expected: all passing.

- [x] **Step 6: Commit**

```bash
git add src/app/api/webhooks/ __tests__/routes/webhooks/stripe.test.ts
git commit -m "feat: add Stripe webhook handler — sub lifecycle + credit grants"
```

---

## Chunk 7: Desktop App Update + Deployment

### Task 19: Update DEFAULT_API_URL in Shepherd desktop

**Files:**
- Modify: `crates/shepherd-core/src/cloud/mod.rs` (in the `SecurityRonin/shepherd` repo, not shepherd-pro)

**This task runs in the `shepherd` repo, not `shepherd-pro`.**

- [x] **Step 1: Update the constant**

In `/Users/4n6h4x0r/src/shepherd/crates/shepherd-core/src/cloud/mod.rs`, change line:
```rust
pub const DEFAULT_API_URL: &str = "https://shepherd-pro.vercel.app";
```
to:
```rust
pub const DEFAULT_API_URL: &str = "https://api.shepherd.codes";
```

- [x] **Step 2: Run Rust tests to confirm nothing breaks**

```bash
cargo test -p shepherd-core --lib
```

Expected: all tests passing (the constant is only used in test assertions — update those too if they hardcode the old URL).

- [x] **Step 3: Check for hardcoded old URL**

```bash
grep -r "shepherd-pro.vercel.app" crates/
```

Expected: no matches.

- [x] **Step 4: Commit**

```bash
git add crates/shepherd-core/src/cloud/mod.rs
git commit -m "chore: update DEFAULT_API_URL to api.shepherd.codes"
```

---

### Task 20: Vercel deployment

- [x] **Step 1: Install Vercel CLI (if not already installed)**

```bash
npm install -g vercel
```

- [x] **Step 2: Link the shepherd-pro repo to Vercel**

From inside the `shepherd-pro` directory:
```bash
vercel link
```

Select "SecurityRonin" as the scope, create a new project named `shepherd-pro`.

- [x] **Step 3: Add environment variables to Vercel**

```bash
vercel env add OPENROUTER_API_KEY production
vercel env add FIRECRAWL_API_KEY production
vercel env add EXA_API_KEY production
vercel env add SUPABASE_URL production
vercel env add SUPABASE_ANON_KEY production
vercel env add SUPABASE_SERVICE_ROLE_KEY production
vercel env add STRIPE_SECRET_KEY production
vercel env add STRIPE_WEBHOOK_SECRET production
vercel env add STRIPE_PRO_PRICE_ID production
vercel env add STRIPE_TOPUP_PRICE_ID production
vercel env add UPSTASH_REDIS_REST_URL production
vercel env add UPSTASH_REDIS_REST_TOKEN production
vercel env add NEXT_PUBLIC_APP_URL production
```

For `NEXT_PUBLIC_APP_URL` enter: `https://api.shepherd.codes`

- [x] **Step 4: Deploy to production**

```bash
vercel --prod
```

- [x] **Step 5: Add custom domain**

```bash
vercel domains add api.shepherd.codes
```

Add the CNAME record shown by Vercel to your DNS provider for `shepherd.codes`.

- [x] **Step 6: Verify deployment**

```bash
curl https://api.shepherd.codes/api/credits/balance
```

Expected: `{"error":"auth_required"}` with status 401 — confirms the route is live and auth is working.

- [x] **Step 7: Configure Stripe webhook**

In the Stripe dashboard → Developers → Webhooks → Add endpoint:
- URL: `https://api.shepherd.codes/api/webhooks/stripe`
- Events: `checkout.session.completed`, `invoice.payment_succeeded`, `invoice.payment_failed`, `customer.subscription.updated`, `customer.subscription.deleted`

Copy the webhook signing secret and update: `vercel env add STRIPE_WEBHOOK_SECRET production`

- [x] **Step 8: Final smoke test**

```bash
curl -X POST https://api.shepherd.codes/api/generate/logo \
  -H "Content-Type: application/json" \
  -d '{"product_name":"Test"}'
```

Expected: `{"error":"auth_required"}` with 401. Confirms route tree is live.

- [x] **Step 9: Commit deployment config**

```bash
git add vercel.json 2>/dev/null; git add -u
git commit -m "chore: Vercel deployment config and final smoke test passing" || echo "nothing to commit"
git push
```

---

## Verification

```bash
# All tests
npm test

# Coverage
npm run test:coverage

# Type check
npx tsc --noEmit

# Build
npm run build
```

Expected: all tests passing, no TypeScript errors, clean build.
