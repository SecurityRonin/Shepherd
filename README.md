<p align="center">
  <img src="docs/assets/shepherd-logo.png" alt="Shepherd" width="120" />
</p>

<h1 align="center">Shepherd</h1>

<p align="center">
  <strong>One screen. Every agent. Full control.</strong><br/>
  The missing command center for developers running Claude Code, Codex, AdaL, OpenCode, Gemini CLI, Aider, Goose, and more — simultaneously.
</p>

<p align="center">
  <a href="https://shepherd.codes">Website</a> ·
  <a href="#get-started-in-60-seconds">Quick Start</a> ·
  <a href="#install">Install</a> ·
  <a href="#features">Features</a> ·
  <a href="#adapters">Adapters</a> ·
  <a href="docs/superpowers/specs/2026-03-10-shepherd-design.md">Design Spec</a> ·
  <a href="docs/competitive-analysis-agent-orchestrators.md">Competitive Analysis</a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/h4x0r"><img src="https://img.shields.io/badge/sponsor-♥-ea4aaa" alt="Sponsor" /></a>
  <img src="https://img.shields.io/badge/status-alpha-orange" alt="Status: Alpha" />
  <img src="https://img.shields.io/badge/license-Apache%202.0-green" alt="License: Apache 2.0" />
  <img src="https://img.shields.io/badge/binary-~600KB-blue" alt="Binary: ~600KB" />
  <img src="https://img.shields.io/badge/platform-macOS%20·%20Linux%20·%20Windows-lightgrey" alt="Platforms" />
</p>

---

You have 12 agents running across 4 repos. One needs approval. Two finished 10 minutes ago and you didn't notice. One crashed silently. You're alt-tabbing between terminal windows, losing context, losing momentum, losing your mind.

You built this workflow. You love these tools. But managing them? That part sucks.

Shepherd fixes that.

---

## Get started in 60 seconds

**Install:**

```bash
# macOS
brew install shepherd-codes/tap/shepherd

# Linux / from source
curl -fsSL https://shepherd.codes/install.sh | sh

# Windows
winget install shepherd-codes.shepherd
```

Installs both `shepherd` and `shep` (same binary, your choice).

**Then run:**

```bash
shep
```

That's it. The GUI opens. What happens next depends on how you work:

---

### Path A — You already have agents running in iTerm2

Shepherd finds them automatically. No config, no restart, no flags.

Every iTerm2 pane running a known agent (Claude Code, Codex, AdaL, Aider, Gemini CLI, OpenCode, Goose, Plandex, gptme) appears on the Kanban board within seconds. Click any card to see the terminal, approve permissions, or review diffs.

**Make adoption persistent across new windows** (30 seconds, one-time):

1. Open iTerm2 → **Preferences → Profiles → General**
2. Under **"Send text at start"**, add: `shepherd-bridge &`

From then on, every new iTerm2 session automatically registers with Shepherd.

---

### Path B — Start a new task from Shepherd

```bash
# From the GUI: click "+ New Task", fill in the form, pick your agent
# Or from the CLI:
shep new "Add rate limiting to the API" \
  --agent claude-code \
  --repo ~/src/myapp \
  --isolation worktree
```

Shepherd spawns the agent in an isolated git worktree, streams its output to the board, and waits for your input when it needs approval.

---

### Daily workflow (once you're set up)

```
1. Open Shepherd (or it's already running in the background)
2. Glance at the board — see what's running, what needs you, what's done
3. Click a "Needs Input" card → approve or deny in one keystroke (⌘ ⏎)
4. When a task reaches "Review" → inspect the diff inline
5. Click "Create PR" → done. Shepherd stages, commits, rebases, and pushes.
```

**Keyboard shortcuts that matter:**

| Key | Action |
|-----|--------|
| `⌘ N` | New task |
| `⌘ ⏎` | Approve current |
| `⌘ ⇧ ⏎` | Approve all pending |
| `⌘ K` | Command palette |
| `⌘ 0` | Toggle overview / focus |

---

## What it does

Shepherd is a native desktop app that watches all your coding agents from a single Kanban board. Approve permissions without switching windows. Review diffs without hunting for the right terminal. Create PRs without touching git.

It works with agents you already use. It doesn't replace them, lock you in, or phone home.

```
┌─────────────┬──────────────┬──────────────┬──────────────┬──────────┐
│   Queued    │   Running    │ Needs Input  │    Review    │   Done   │
├─────────────┼──────────────┼──────────────┼──────────────┼──────────┤
│ Add search  │ ◉ Refactor   │ ⚠ Auth flow  │ ✓ API tests │ ✓ Docs   │
│   Claude    │   db module  │   wants to   │   12 files   │   Done   │
│             │   Codex      │   rm -rf tmp │   +340 -89   │   2h ago │
│ Fix #412    │              │              │              │          │
│   Gemini    │ ◉ Landing pg │  ✓ Approve   │  View Diff   │          │
│             │   Aider      │              │              │          │
└─────────────┴──────────────┴──────────────┴──────────────┴──────────┘
```

Click a card to drill down: full terminal, live diff viewer, inline comments that feed back into the agent's prompt.

## Why this exists

We looked at [22+ tools in this space](docs/competitive-analysis-agent-orchestrators.md). Here's what we found:

**Kanban tools** (Vibe Kanban, Emdash) give you visual oversight but no safety rails. They run everything in YOLO mode by default. No rules engine. No quality gates.

**Terminal tools** (Clorch, Claude Squad, Mato) give you a rules engine and hotkeys but no visual diff review, no Kanban, no PR workflow.

**IDE tools** (Cursor, Cline, JetBrains Air) bolt agents onto an editor. You get one agent per window. Managing 10 agents means 10 windows.

**Cloud tools** (Devin, Factory) cost $500/month and run your code on someone else's machine.

No single tool combines Kanban + rules engine + quality gates + multi-agent + one-click PRs. Shepherd does.

## Features

### Agent-agnostic from day one

Nine first-class agents, plus iTerm2 session adoption for agents already running in your terminal. Drop a TOML file to add any other terminal-based agent.

| Agent | Status | Hook Protocol | iTerm2 Adoption |
|-------|--------|---------------|-----------------|
| Claude Code | First-class | Native hooks | ✓ |
| Codex CLI | First-class | Output parsing | ✓ |
| AdaL | First-class | Output parsing | ✓ (featured partner) |
| OpenCode | First-class | Output parsing | ✓ |
| Gemini CLI | First-class | Output parsing | ✓ |
| Aider | First-class | Output parsing | ✓ |
| Goose | First-class | Output parsing | ✓ |
| Plandex | First-class | Output parsing | ✓ |
| gptme | First-class | Output parsing | ✓ |
| Your agent | [Write a TOML](docs/adapters.md) | Output parsing | — |

### YOLO mode with a brain

Three permission levels per task or globally:

- **Ask** (default): every tool call needs your approval
- **Smart**: auto-approve reads and safe writes, ask for deletes and system commands
- **YOLO**: auto-approve everything except deny-listed patterns

The rules engine uses YAML. First match wins. Deny rules override everything.

```yaml
# ~/.shepherd/rules.yaml
deny:
  - pattern: "rm -rf /"
  - pattern: "git push --force"
  - tool: "Bash"
    pattern: "curl.*| sh"

allow:
  - tool: "Read"
  - tool: "Glob"
  - tool: "Write"
    path: "src/**"
```

Every action is logged. Auto-approved or manual. Full audit trail in SQLite.

**Kernel-level guardrails via [nono.sh](https://nono.sh):** Even in YOLO mode, agents run inside a kernel-enforced sandbox. SSH keys, AWS credentials, and shell configs are blocked by default. Child processes inherit all restrictions. No API to escape, not even for nono itself.

### iTerm2 Session Adoption

Already have agents running in iTerm2? Shepherd finds them automatically — no restart required.

On launch, Shepherd scans every iTerm2 pane, detects which ones are running a known coding agent (by process name), and pulls them into the Kanban board as live tasks. You get full oversight of sessions you started before Shepherd was running.

```
iTerm2 window                    Shepherd Kanban
─────────────────                ─────────────────
  pane 1: claude  ──────────▶   [claude-code: ~/src/api]   🟣 iTerm2
  pane 2: adal    ──────────▶   [adal: ~/src/frontend]     🟣 iTerm2
  pane 3: aider   ──────────▶   [aider: ~/src/infra]       🟣 iTerm2
  pane 4: vim          ✗        (not a coding agent)
```

**For Claude Code sessions**, Shepherd offers a session picker on the task card: choose a previous Claude Code session to resume (sorted newest-first by mtime) or start fresh. No `--continue` flags to remember.

**For all other agents**, tasks appear immediately in the board with a purple "iTerm2" badge and the detected agent name.

Setup takes 30 seconds: add `shepherd-bridge.py` to your iTerm2 AutoLaunch profile (`Preferences → Profiles → General → Send text at start`). The bridge forwards your iTerm2 cookie and key to Shepherd's auth file so session scanning works without manual configuration.

### Quality gates that block bad PRs

When a task enters Review, gates run automatically:

- **Lint**: eslint, ruff, clippy (auto-detected)
- **Format**: prettier, black, rustfmt
- **Type check**: tsc, mypy, cargo check
- **Tests**: runs affected tests only
- **Security**: Sonar, Semgrep, Snyk (plugin)
- **Custom**: drop scripts in `.shepherd/gates/`

Failing gates block the PR button. One click tells the agent to fix them.

### One-click PR pipeline

Done reviewing a task? Click "Create PR":

1. Stage changes in the task's worktree
2. LLM generates a commit message from the agent's work
3. You edit or accept
4. Rebase on latest main
5. Quality gates run
6. Push branch
7. `gh pr create` with auto-generated body
8. Worktree cleaned up

Zero git commands. Zero context switches.

### Three isolation modes

| Mode | Speed | Safety | Use case |
|------|-------|--------|----------|
| Git Worktree | Fast | Good | Default. Most tasks. |
| Docker Container | Medium | Strong | Untrusted code, experiments |
| Local Workspace | Instant | None | Quick fixes on current branch |

### Keyboard-first

| Shortcut | Action |
|----------|--------|
| `⌘ 0` | Toggle Overview / Focus |
| `⌘ N` | New Task |
| `⌘ ⏎` | Approve current |
| `⌘ ⇧ ⏎` | Approve all pending |
| `⌘ ]` / `⌘ [` | Next / previous session |
| `⌘ K` | Command palette |
| `1-9` | Quick-approve card N |

### CLI for when you'd rather type

```bash
shep                        # start server + open GUI
shep status                 # 3 running · 1 needs input · 2 done  (alias: shep s)
shep new "task"             # create task (--agent, --isolation flags)
shep approve 4              # approve task #4                      (alias: shep a)
shep approve --all          # approve all pending
shep pr 7                   # create PR for task #7
shep pr 7 --base dev        # PR against a different branch
shep gates 7                # run quality gates for task #7
shep namegen "AI tool"      # brainstorm product names from CLI
shep init                   # init .shepherd/ in current project
shep stop                   # stop server
shep completions bash       # generate shell completions
```

GUI and CLI share the same server. Same state. Use both.

### Notifications that respect your flow

- **macOS native notifications** with an Approve action button (approve without opening the app)
- **Distinct sounds** for permission requests, completions, and errors
- **Dock badge** showing tasks needing input
- **Menu bar icon** (green/orange/red) with summary dropdown
- **Staleness indicators**: yellow after 30s idle, red after 2min

### Lifecycle tools no other orchestrator has

**New Project Wizard**: guided journey from zero to shipping — North Star strategy, brand name with domain validation, logo generation, and Superpowers setup. Skip any step, jump anywhere, dismiss entirely. Always optional, never forced.

**Product Name Generator**: describe your product, get 20+ candidates with automatic RDAP domain checks, npm, PyPI, and GitHub conflict validation. All-clear names sorted first. Available from CLI: `shep namegen "AI productivity tool" --vibes bold minimal`.

**Logo Generator**: pick a style (minimal, geometric, mascot, abstract), get 4 variants, auto-export to PNG (all sizes), favicon.ico, apple-touch-icon, macOS .icns, Windows .ico, and web manifest.

**Full-stack SDD (Spec-Driven Development)**: powered by [North Star Advisor](https://northstaradvisor.app/) + [Obra Superpowers](https://github.com/obra/superpowers). North Star generates strategic foundations (brand guidelines, competitive landscape, user personas, architecture blueprints) across 13 analysis phases. Obra Superpowers turns ideas into specs through structured brainstorming, then specs into bite-sized TDD plans. Output feeds into every future agent session as context. Strategy to spec to plan to code.

**Contextual Triggers**: Shepherd detects when your project is missing a name, logo, or strategy docs and suggests the right tool via non-intrusive toast notifications. Dismiss once and it won't come back.

**Ecosystem Auto-Install**: auto-detects and offers to install [Obra Superpowers](https://github.com/obra/superpowers), [context-mode](https://github.com/mksglu/context-mode), and [Alaya](https://github.com/SecurityRonin/alaya) into each agent's config. Superpowers and context-mode drop into `~/.claude/`, `~/.codex/`, `~/.opencode/`. Alaya is injected as an MCP server entry into `~/.claude.json` — preferring a local build at `~/src/alaya/target/release/alaya-mcp` if present, otherwise pulling from npm. Shepherd manages the meta-layer; agents stay native.

## Architecture

```
FRONTEND (Tauri 2.0 · ~600KB binary)
├── React + TypeScript + Zustand + TailwindCSS
├── xterm.js (terminal emulation)
└── Monaco Editor (diff viewer)
    │
    │ WebSocket (real-time) + REST (commands)
    ▼
BACKEND (Rust · localhost)
├── PTY Manager      — spawn/kill agents, stream output, nono.sh sandbox
├── Agent Adapters   — TOML-defined, 6 first-class + community
├── YOLO Engine      — YAML rules, deny/allow, audit log
├── Quality Gates    — lint/format/type/test + plugin gates
├── LLM Client       — OpenAI, Anthropic, Ollama (provider-agnostic)
├── Lifecycle Tools  — name gen, logo gen, North Star PMF, wizard
├── Ecosystem        — Superpowers + context-mode + Alaya MCP auto-install
├── iTerm2 Manager   — session adoption, jobName detection, 9 agents
├── Triggers         — contextual suggestions (name, logo, strategy)
├── PR Pipeline      — stage, commit, rebase, gates, push, gh pr create
└── State (SQLite)   — tasks, sessions, permissions, diffs, gate results
    │
    ▼
AGENTS (your existing tools, unchanged)
├── Claude Code    ├── Codex CLI    ├── AdaL
├── OpenCode       ├── Gemini CLI   ├── Aider
├── Goose          ├── Plandex      ├── gptme
└── community adapters (TOML)
```

**Why Tauri over Electron**: ~600KB vs ~150MB. Native performance. Rust backend. No bundled Chromium.

**Why SQLite**: single file, WAL mode, fast concurrent reads. Your data stays on your machine.

**Why TOML adapters**: adding a new agent means writing 20 lines of config, not a plugin SDK.

<h2 id="install">Install</h2>

See **[Get started in 60 seconds](#get-started-in-60-seconds)** at the top for the full quickstart.

```bash
# macOS
brew install shepherd-codes/tap/shepherd

# Linux
curl -fsSL https://shepherd.codes/install.sh | sh

# Windows
winget install shepherd-codes.shepherd

# From source (installs both `shepherd` and `shep`)
git clone https://github.com/SecurityRonin/Shepherd.git
cd Shepherd && bash scripts/install.sh && npm install && npm run build
```

Both `shepherd` and `shep` are installed — they're the same binary. Use whichever you prefer. Most examples in this README use `shep`.

<h2 id="adapters">Writing a custom adapter</h2>

Any terminal-based agent works. Create a TOML file:

```toml
# ~/.shepherd/adapters/my-agent.toml

[agent]
name = "My Agent"
command = "my-agent"
args = ["--auto"]
version_check = "my-agent --version"

[status]
working_patterns = ["Generating", "Writing"]
idle_patterns = ["$ ", "> "]
input_patterns = ["[y/n]", "approve?"]
error_patterns = ["Error:", "FAILED"]

[permissions]
approve = "y\n"
deny = "n\n"
```

Restart Shepherd. Your agent shows up in the New Task dropdown.

## Configuration

```
~/.shepherd/
├── config.toml        # global settings
├── rules.yaml         # YOLO rules
├── adapters/          # custom agent adapters
└── db.sqlite          # local database

.shepherd/             # per-project (optional)
├── config.toml        # project overrides
├── rules.yaml         # project-specific rules
└── gates/             # custom quality gate scripts
```

## Comparison

| | Shepherd | Vibe Kanban | Clorch | Claude Squad | Emdash | JetBrains Air |
|---|:---:|:---:|:---:|:---:|:---:|:---:|
| Multi-agent (6+ providers) | ✓ | ✓ | Claude only | 5 agents | 20+ agents | 4 agents |
| Kanban board | ✓ | ✓ | ✗ | ✗ | ✓ | ✗ |
| YOLO rules engine | ✓ | YOLO only | ✓ | ✗ | ✗ | 4-level |
| Quality gates | ✓ | Plugin | ✗ | ✗ | ✗ | ✗ |
| One-click PR | ✓ | ✓ | ✗ | ✗ | ✓ | ✗ |
| CLI + GUI | ✓ | ✗ | CLI only | CLI only | ✗ | ✗ |
| Shell completions | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Kernel sandbox (nono.sh) | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Diff review + comments | ✓ | ✓ | ✗ | ✗ | ✓ | ✓ |
| Cross-platform | ✓ | ✓ | macOS | ✓ | ✓ | macOS |
| Binary size | ~600KB | ~150MB | pip | Go binary | ~150MB | ~500MB |
| Name gen + logo gen + [North Star](https://northstaradvisor.app/) PMF | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ |
| New project wizard | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Ecosystem auto-install | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Open source | Apache 2.0 | Apache 2.0 | MIT | MIT | MIT | ✗ |

## Roadmap

**v1.0** (current): Core engine, Kanban board, 9 first-class agents, YOLO engine, quality gates, PR pipeline, CLI with shell completions, LLM client (OpenAI/Anthropic/Ollama), name generator, logo generator, North Star PMF wizard, contextual triggers, nono.sh sandbox, ecosystem auto-install (Superpowers + context-mode + Alaya), new project wizard, iTerm2 session adoption (9 agents, session picker, permission prompt detection, bridge script). 1,100+ tests.

**v1.1**: Best-of-N (run same task on multiple agents, compare outputs). Issue tracker integration (Linear, GitHub Issues, Jira). Event-driven automations.

**v2.0**: Mobile monitoring (push notifications, approve from phone). Team dashboards. Browser UI for remote access. Adapter registry.

## Contributing

Shepherd is Apache 2.0 licensed and built in the open.

```bash
git clone https://github.com/SecurityRonin/Shepherd.git
cd Shepherd
cargo build
npm install && npm run dev
```

PRs welcome. Check [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

If you find Shepherd useful, star the repo. It helps others find it.

## Powered by full-stack SDD (Spec-Driven Development)

- [North Star Advisor](https://northstaradvisor.app/) for product-market fit and strategic planning
- [Obra Superpowers](https://github.com/obra/superpowers) for brainstorming, planning, and agentic development workflows
- [context-mode](https://github.com/mksglu/context-mode) for intelligent context window management and token optimization
- [Alaya](https://github.com/SecurityRonin/alaya) for persistent episodic/semantic memory across agent sessions
- [nono.sh](https://nono.sh) for kernel-level agent sandboxing (Landlock on Linux, Seatbelt on macOS)

Strategy to spec to plan to code. Every token counts. No agent escapes the sandbox.

## License

Apache 2.0. See [LICENSE](LICENSE) for details.
