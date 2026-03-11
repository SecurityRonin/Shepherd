<p align="center">
  <img src="docs/assets/shepherd-logo.png" alt="Shepherd" width="120" />
</p>

<h1 align="center">Shepherd</h1>

<p align="center">
  <strong>One screen. Every agent. Full control.</strong><br/>
  The missing command center for developers running Claude Code, Codex, Gemini CLI, OpenCode, and Aider at the same time.
</p>

<p align="center">
  <a href="https://shepherd.codes">Website</a> ·
  <a href="#install">Install</a> ·
  <a href="#features">Features</a> ·
  <a href="#adapters">Adapters</a> ·
  <a href="docs/superpowers/specs/2026-03-10-shepherd-design.md">Design Spec</a> ·
  <a href="docs/competitive-analysis-agent-orchestrators.md">Competitive Analysis</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/status-alpha-orange" alt="Status: Alpha" />
  <img src="https://img.shields.io/badge/license-Apache%202.0-green" alt="License: MIT" />
  <img src="https://img.shields.io/badge/binary-~600KB-blue" alt="Binary: ~600KB" />
  <img src="https://img.shields.io/badge/platform-macOS%20·%20Linux%20·%20Windows-lightgrey" alt="Platforms" />
</p>

---

You have 12 agents running across 4 repos. One needs approval. Two finished 10 minutes ago and you didn't notice. One crashed silently. You're alt-tabbing between terminal windows, losing context, losing momentum, losing your mind.

You built this workflow. You love these tools. But managing them? That part sucks.

Shepherd fixes that.

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

Five first-class adapters. Drop a TOML file to add any terminal-based agent.

| Agent | Status | Hook Protocol |
|-------|--------|---------------|
| Claude Code | First-class | Native hooks |
| Codex CLI | First-class | Output parsing |
| Gemini CLI | First-class | Output parsing |
| OpenCode | First-class | Output parsing |
| Aider | First-class | Output parsing |
| Your agent | [Write a TOML](docs/adapters.md) | Output parsing |

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
shepherd              # start server + open GUI
shepherd status       # 3 running · 1 needs input · 2 done
shepherd new "task"   # create task (--agent, --isolation flags)
shepherd approve 4    # approve task #4
shepherd approve --all
shepherd pr 7         # create PR for task #7

alias shep=shepherd
shep s               # status
shep a               # approve next
shep aa              # approve all
```

GUI and CLI share the same server. Same state. Use both.

### Notifications that respect your flow

- **macOS native notifications** with an Approve action button (approve without opening the app)
- **Distinct sounds** for permission requests, completions, and errors
- **Dock badge** showing tasks needing input
- **Menu bar icon** (green/orange/red) with summary dropdown
- **Staleness indicators**: yellow after 30s idle, red after 2min

### Lifecycle tools no other orchestrator has

**Product Name Generator**: describe your product, get 20+ candidates with automatic WHOIS, npm, PyPI, and GitHub conflict checks. All-clear names sorted first.

**Logo Generator**: pick a style (minimal, geometric, mascot, abstract), get 4 variants, auto-export to SVG, PNG (all sizes), favicon, apple-touch-icon, macOS .icns, Windows .ico.

**North Star PMF**: powered by [North Star Advisor](https://northstaradvisor.app/), a 13-phase strategic wizard generating brand guidelines, competitive landscape, user personas, architecture blueprints. Output feeds into every future agent session as context.

**Brainstorming**: powered by [Obra Superpowers](https://obra.ai), collaborative design sessions that turn ideas into fully formed specs through structured dialogue, approach exploration, and incremental validation before a single line of code is written.

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
├── PTY Manager      — spawn/kill agents, stream output
├── Agent Adapters   — TOML-defined, 5 first-class + community
├── YOLO Engine      — YAML rules, deny/allow, audit log
├── Quality Gates    — lint/format/type/test + plugins
├── State (SQLite)   — tasks, sessions, permissions, diffs
└── Lifecycle Tools  — name gen, logo gen, North Star PMF
    │
    ▼
AGENTS (your existing tools, unchanged)
├── Claude Code    ├── Codex CLI
├── Gemini CLI     ├── OpenCode
├── Aider          └── community adapters
```

**Why Tauri over Electron**: ~600KB vs ~150MB. Native performance. Rust backend. No bundled Chromium.

**Why SQLite**: single file, WAL mode, fast concurrent reads. Your data stays on your machine.

**Why TOML adapters**: adding a new agent means writing 20 lines of config, not a plugin SDK.

<h2 id="install">Install</h2>

```bash
# macOS (Homebrew)
brew install shepherd-codes/tap/shepherd

# Linux
curl -fsSL https://shepherd.codes/install.sh | sh

# Windows
winget install shepherd-codes.shepherd

# From source
git clone https://github.com/shepherd-codes/shepherd
cd shepherd && cargo build --release
```

## Quick start

```bash
# 1. Start Shepherd
shepherd

# 2. Create your first task
shepherd new "Add user authentication to the API" \
  --agent claude-code \
  --repo ~/src/myapp \
  --isolation worktree

# 3. Watch it work from the Kanban board
# 4. Approve permissions as they come in
# 5. Review diffs when it's done
# 6. One-click PR
```

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
| Multi-agent (5+ providers) | ✓ | ✓ | Claude only | 5 agents | 20+ agents | 4 agents |
| Kanban board | ✓ | ✓ | ✗ | ✗ | ✓ | ✗ |
| YOLO rules engine | ✓ | YOLO only | ✓ | ✗ | ✗ | 4-level |
| Quality gates | ✓ | Plugin | ✗ | ✗ | ✗ | ✗ |
| One-click PR | ✓ | ✓ | ✗ | ✗ | ✓ | ✗ |
| CLI + GUI | ✓ | ✗ | CLI only | CLI only | ✗ | ✗ |
| Diff review + comments | ✓ | ✓ | ✗ | ✗ | ✓ | ✓ |
| Cross-platform | ✓ | ✓ | macOS | ✓ | ✓ | macOS |
| Binary size | ~600KB | ~150MB | pip | Go binary | ~150MB | ~500MB |
| Name/logo/[North Star](https://northstaradvisor.app/) PMF | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Open source | Apache 2.0 | Apache 2.0 | MIT | MIT | MIT | ✗ |

## Roadmap

**v1.0** (current): Core engine, Kanban board, 5 adapters, YOLO engine, quality gates, PR pipeline, CLI.

**v1.1**: Best-of-N (run same task on multiple agents, compare outputs). Issue tracker integration (Linear, GitHub Issues, Jira). Event-driven automations.

**v2.0**: Mobile monitoring (push notifications, approve from phone). Team dashboards. Browser UI for remote access. Adapter registry.

## Contributing

Shepherd is Apache 2.0 licensed and built in the open.

```bash
git clone https://github.com/shepherd-codes/shepherd
cd shepherd
cargo build
cd frontend && npm install && npm run dev
```

PRs welcome. Check [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

If you find Shepherd useful, star the repo. It helps others find it.

## Powered by

- [North Star Advisor](https://northstaradvisor.app/) for product-market fit and strategic planning
- [Obra Superpowers](https://obra.ai) for brainstorming, planning, and agentic development workflows

## License

Apache 2.0. See [LICENSE](LICENSE) for details.
