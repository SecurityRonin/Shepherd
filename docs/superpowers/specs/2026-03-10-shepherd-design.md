# Shepherd — Design Specification

**Product:** Shepherd (shepherd.codes)
**Date:** 2026-03-10
**Status:** Approved

A cross-platform desktop GUI for managing multiple AI coding agents. Combines the best features from 40+ competing tools (Broomy, Kintsugi, Clorch, Air, Claude Squad, Vibe Kanban, Emdash, and more) with unique end-to-end lifecycle features no competitor offers.

## Target User

Vibe coders and context engineers familiar with the Claude Code workflow. Users who run 5-20 concurrent coding agents across multiple repos and need situational awareness without cognitive overload.

## Core Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Name | Shepherd (shepherd.codes) | Leadership + guidance metaphor. Domain available. |
| Framework | Tauri 2.0 | ~600KB vs Electron's ~150MB. Rust backend + web frontend. |
| Architecture | Local Rust server + Tauri shell | Engine/UI separation. Desktop now, web/mobile later. |
| Agent support | Hybrid | First-class adapters for top 5 + open adapter spec. |
| Layout | Kanban overview + panel drill-down | Two modes: situational awareness + deep interaction. |
| Lifecycle tools | Contextual triggers | Non-intrusive suggestions when relevant. Always available via ⌘K. |
| Scope | Profiles (invisible by default) | Default profile = everything. Power users discover profiles later. |

## System Architecture

### Three-Layer Architecture

```
FRONTEND (Tauri 2.0 Shell)
├── React + TypeScript
├── xterm.js (embedded terminals)
├── Monaco Editor (diff viewer)
├── Zustand (state management)
└── TailwindCSS
    │
    │ WebSocket (real-time) + REST (commands)
    ▼
BACKEND (Rust Server · localhost:{port})
├── PTY Manager — spawn/kill agent processes, output capture
├── Hook Engine — install hooks, file-based state, event normalization
├── Agent Adapters — TOML-defined per agent, first-class + community
├── Git Operations — worktree mgmt, branch tracking, staging, PR creation
├── State Manager — SQLite (WAL mode), tasks, sessions, profiles, audit
├── Quality Gates — lint/format/type/test + plugin gates (Sonar, custom)
├── Notification System — macOS native, sounds, dock badge, menu bar
├── YOLO Engine — auto-approve rules (YAML), deny patterns, audit log
└── Lifecycle Tools — name generator, logo generator, North Star PMF
    │
    │ Process spawn / Hooks / File I/O
    ▼
AGENT LAYER
├── Claude Code (first-class, native hooks)
├── Codex CLI (first-class, output parsing)
├── OpenCode (first-class, output parsing)
├── Gemini CLI (first-class, output parsing)
├── Aider (first-class, output parsing)
└── Community adapters (TOML spec)
    │
    │ Isolation modes (per-task)
    ├── Git Worktree (default, lightweight)
    ├── Docker Container (sandboxed)
    └── Local Workspace (no isolation)
```

### Data Flow: Task Lifecycle

1. **User creates task** — name, agent, repo, isolation mode, optional prompt
2. **Shepherd Core** — creates worktree, spawns agent PTY, installs hooks, begins output capture
3. **Task status: Queued → Running** — Kanban card appears in Running column with live action text
4. **Agent needs permission** — hook intercepts, YOLO engine checks rules, auto-approve or surface to UI
5. **If manual approval needed** — task moves to Needs Input, Kanban card shows ✓ Approve button, macOS notification + sound
6. **Agent completes** — task moves to Review, diff snapshots generated, quality gates run
7. **User reviews and approves** — one-click PR pipeline: stage → commit → rebase → gates → push → PR
8. **Task status: Done** — card fades, auto-collapses after 24h

### Key Technical Decisions

- **State:** SQLite with WAL mode. Single file, fast concurrent reads. Stores sessions, tasks, profiles, audit log, settings.
- **IPC:** WebSocket for real-time state pushes (terminal output, status changes). REST for commands (create task, approve, settings).
- **Config:** Global at `~/.shepherd/`, per-project at `.shepherd/` in repo root. TOML for config, YAML for rules.
- **CLI:** `shepherd` command (alias: `shep`) queries local server API. Headless mode for CI/scripting.

## UI/UX Design

### Mode 1 — Overview (Kanban Home)

The Kanban board is the home screen. All agents visible at a glance.

**Columns:**
- **Queued** — tasks waiting. Drag to reorder priority.
- **Running** — active agents. Shows current action (e.g., "Editing src/db/pool.ts").
- **Needs Input** — permission requests. **✓ Approve button on each card** (approve-only; no reject button — drill down for that). This is the key cognitive load optimization: approving is low-context, rejecting is high-context.
- **Review** — agent finished. Shows file count + diff stats. Quality gate badges.
- **Done** — completed. Faded opacity. Auto-collapses after 24h.

**Card contents:** Task name, agent type badge, branch name, current action/permission question/diff stats.

### Mode 2 — Focus (Panel Drill-Down)

Click any Kanban card to enter Focus mode.

**Three-panel layout:**
- **Session Sidebar (left, 180px)** — all sessions with status dots. Click to switch. "← Overview" at top.
- **Agent Terminal (center)** — full xterm.js terminal. Permission prompt: Approve / Approve All / text input. "Pop out" to detach.
- **Changes Panel (right)** — file tabs, unified or side-by-side diff. Click any line → inline comment that feeds back to agent prompt.

**Task Header Bar:** Task name, agent badge, branch, isolation mode, time since last activity, status badge.

### Permission Management (YOLO Mode)

Three permission levels (per-task or global):

- **Ask** — every tool request requires approval (default)
- **Smart** — auto-approve reads and safe writes, ask for deletes/system commands. Uses YOLO rules engine.
- **YOLO** — auto-approve everything except deny-listed patterns

**Rules engine** (`~/.shepherd/rules.yaml`):

```yaml
# First match wins
deny:
  - pattern: "rm -rf /"
  - pattern: "git push --force"
  - pattern: "DROP TABLE"
  - tool: "Bash"
    pattern: "curl.*| sh"

allow:
  - tool: "Read"
  - tool: "Glob"
  - tool: "Grep"
  - tool: "Write"
    path: "src/**"
```

Every action (auto-approved or manual) logged to SQLite audit table.

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `⌘ 0` | Toggle Overview / Focus |
| `⌘ N` | New Task |
| `⌘ ⏎` | Approve current |
| `⌘ ⇧ ⏎` | Approve all pending |
| `⌘ ]` / `⌘ [` | Next / previous session |
| `⌘ 1` | Focus terminal |
| `⌘ 2` | Focus changes |
| `⌘ K` | Command palette |
| `1-9` | Quick approve card N (in Overview) |

### Notification & Attention System

- **macOS native notifications** with Approve action button
- **System sounds** — distinct for permission/complete/error (configurable)
- **Dock badge** — count of tasks needing input
- **Menu bar icon** — green/orange/red status. Click for summary dropdown.
- **Staleness indicators** — yellow >30s, red >2min idle

## End-to-End Lifecycle Features (Unique Differentiators)

### Contextual Triggers

Shepherd detects when lifecycle tools would be useful and suggests them via non-intrusive toast. Never forced, always dismissable, never shown twice for the same suggestion. All tools also accessible via ⌘K command palette.

| Tool | Trigger Condition | Toast |
|------|------------------|-------|
| Name Generator | No package.json name, or name is "untitled" | "Want help brainstorming a product name?" |
| Logo Generator | No favicon.ico, no app icon detected | "No app icon found. Generate a logo?" |
| North Star PMF | No docs/ directory, no strategy docs | "Define your product strategy?" |

### Product Name Generator

1. User provides product description + optional "vibe" tags
2. LLM brainstorms 20+ name candidates
3. Each candidate validated automatically:
   - WHOIS domain availability (5+ TLDs via RDAP)
   - npm/PyPI registry checks
   - GitHub org/repo conflicts
   - Negative association scan (LLM-powered, multilingual)
4. Results sorted: all-clear first, partial second, conflicted crossed out
5. User selects or regenerates with adjusted vibes

### Logo & Icon Generator

1. User selects style (minimal / geometric / mascot / abstract) + optional colors
2. Image generation API call (BYOK: DALL-E, Midjourney, Stable Diffusion, Flux)
3. 4 variants generated. User picks one or regenerates.
4. Auto-export to all required sizes and formats:
   - SVG (vector source)
   - PNG: 1024, 512, 192, 64
   - favicon.ico, apple-touch-icon
   - manifest.json icons array
   - macOS .icns, Windows .ico
5. Files placed in project `public/` or `assets/`, manifest updated

### North Star PMF Integration

Integrates the North Star Advisor methodology. 13-phase wizard generating up to 22 strategic documents.

**Phases:** Brand guidelines → North Star metric → Competitive landscape (4 parallel research agents) → User personas → User journeys → UI design system → Architecture blueprint → Security architecture → ADRs → Action roadmap → Strategic recommendation

**Key integration points:**
- Generated `ai-context.yml` referenced in agent config — every future agent session inherits strategic context
- Kill list items become guardrails — agents flagged if proposing killed features
- Architecture blueprint informs worktree structure and agent assignment
- Success metrics displayed on Shepherd dashboard

### Quality Gates

Run automatically when a task enters Review state.

**Built-in:** Lint (eslint/ruff/clippy, auto-detected), format check, type check, test runner (affected tests).

**Plugin:** Security scan (Sonar/Semgrep/Snyk), custom scripts (`.shepherd/gates/`), AI code review, license compliance.

Failing gates block the one-click PR workflow. Agent can be asked to fix failures automatically.

## Agent Adapter Protocol (SAP)

Every agent defined by a TOML adapter file.

```toml
[agent]
name = "Claude Code"
command = "claude"
args = ["--dangerously-skip-permissions"]  # YOLO mode
args_interactive = []                       # Ask mode
version_check = "claude --version"
icon = "claude"

[hooks]
type = "claude-code"      # native hook protocol
install = "auto"          # Shepherd manages installation
state_dir = "~/.shepherd/state/"

[status]
working_patterns = ["Reading ", "Writing ", "Editing ", "Searching "]
idle_patterns = ["╰─", "$ "]
input_patterns = ["[y/n", "Permission", "? "]
error_patterns = ["Error:", "FAILED", "panic"]

[permissions]
approve = "y\n"
approve_all = "Y\n"
deny = "n\n"

[capabilities]
supports_hooks = true
supports_prompt_arg = true
supports_resume = true
supports_mcp = true
supports_worktree = true
```

**First-class adapters (v1.0):** Claude Code, Codex CLI, OpenCode, Gemini CLI, Aider.

**Community adapters:** Drop TOML file into `~/.shepherd/adapters/`. Future: public adapter registry.

## Git Workflow — One-Click PR Pipeline

When user clicks "Create PR" on a completed task:

1. Stage all changes in task's worktree
2. LLM generates commit message from agent's work summary
3. User can edit or accept
4. Pull latest main, rebase if needed
5. Run quality gates
6. Push branch
7. Create PR via `gh pr create` with auto-generated body (summary, changes, gate results)
8. Clean up worktree (optional, configurable)

## CLI Interface

```bash
shepherd              # Start server + open GUI
shepherd status       # 3 running · 1 needs input · 2 done
shepherd new "task"   # Create task (--agent, --isolation flags)
shepherd approve 4    # Approve task #4's pending permission
shepherd approve --all # Approve all pending
shepherd pr 7         # Create PR for task #7
shepherd init         # Install hooks, create .shepherd/

# Alias for speed
alias shep=shepherd
shep s   # status
shep a   # approve next
shep aa  # approve all
```

## Project Configuration

**Global:** `~/.shepherd/`
- `config.toml` — global settings
- `rules.yaml` — YOLO rules
- `adapters/` — agent adapter TOML files
- `profiles/` — profile configs (invisible by default)
- `db.sqlite` — sessions, tasks, audit log
- `keys.toml` — API keys (encrypted)

**Per-project:** `.shepherd/`
- `config.toml` — project overrides (default agent, isolation, branch prefix)
- `rules.yaml` — project-specific YOLO rules
- `gates/` — custom quality gate scripts
- `review.md` — code review template

## Competitive Advantages

| Feature | Shepherd | Broomy | Kintsugi | Clorch | Air | Claude Squad |
|---------|----------|--------|----------|--------|-----|-------------|
| Multi-agent orchestration | ✓ | ✓ | ✗ (Claude only) | ✗ (Claude only) | ✓ | ✓ (Claude only) |
| Kanban auto-tracking | ✓ | ✗ | ✓ | ✗ | ✗ | ✗ |
| YOLO mode + safety rules | ✓ | ✗ | ✗ | ✓ | Partial | ✗ |
| Quality gates | ✓ | ✗ | ✓ (Sonar) | ✗ | ✗ | ✗ |
| Diff review + inline comments | ✓ | ✓ | ✓ | ✗ | ✓ | ✗ |
| Product name generator | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Logo/icon generator | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ |
| North Star PMF | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ |
| External terminal monitoring | ✓ | ✗ | ✓ | ✓ | ✗ | ✗ |
| One-click PR pipeline | ✓ | ✓ | ✗ | ✗ | ✗ | ✗ |
| CLI interface | ✓ | ✗ | ✗ | ✓ | ✗ | ✓ |
| 3-tier isolation | ✓ | Docker only | ✗ | ✗ | ✓ | ✗ |
| Cross-platform | ✓ (Tauri) | Experimental | ✗ (macOS) | ✗ (macOS) | ✗ (macOS) | ✓ |
| Lightweight binary | ✓ (~600KB) | ✗ (~150MB) | ✓ | ✓ | ✗ (~1GB) | ✓ |

## Error Handling & Recovery

### Agent Crash Recovery

- **Crash detection:** PTY process exit with non-zero code, or no output for configurable timeout (default: 5min)
- **Auto-restart:** Configurable per-agent. Default: prompt user. Option: auto-restart up to N times with exponential backoff.
- **State preservation:** Last 1000 lines of terminal output saved to SQLite before PTY cleanup. Task moves to "Error" state (red) on Kanban.
- **Worktree safety:** Crashed agent's worktree is never auto-deleted. User can inspect, manually fix, or assign to a new agent.
- **Hung agent detection:** If agent produces no output for >5min (configurable), Shepherd shows a warning badge. User can kill, restart, or wait.

### Concurrency & Resource Management

- **Default concurrent limit:** 10 agents (configurable in `config.toml`)
- **Queue behavior:** Tasks beyond the limit stay in Queued column. FIFO scheduling. Priority override via drag-and-drop.
- **Resource monitoring:** Shepherd tracks per-agent CPU and memory usage via process stats. Warning toast if total agent memory exceeds configurable threshold (default: 8GB).
- **Graceful shutdown:** `shepherd stop` sends SIGTERM to all agents, waits 10s, then SIGKILL. Session state saved for resume.

### Git Conflict Resolution

- **Rebase conflicts during PR pipeline:** Pipeline pauses, task moves to Needs Input with "Merge conflict in X files" message. User can resolve in Focus mode terminal or abort.
- **Worktree conflicts between agents:** Each agent gets its own worktree by default — conflicts are impossible. If two tasks use the same branch (user override), Shepherd warns at task creation.

## Cross-Platform Strategy

- **macOS:** Full feature set including native notifications, dock badge, menu bar icon, system sounds
- **Linux:** Notifications via `notify-send` / D-Bus. Tray icon via system tray protocol. Sounds via `paplay` / PulseAudio.
- **Windows:** Notifications via Windows Toast API. System tray icon. Sounds via Windows audio API.
- **Platform abstraction:** Notification/sound/tray module behind a trait in Rust. Each platform implements the trait. Tauri 2.0 provides some of this out of the box.
- **Launch priority:** macOS first (target audience), Linux second, Windows third.

## YOLO Rules Engine Detail

- **Default policy:** deny (if no rule matches, permission is required)
- **Pattern syntax:** glob patterns for paths, regex for command/content matching
- **Rule precedence:** deny rules checked first, then allow rules. First match wins within each category.
- **Profiles scope:** Global rules in `~/.shepherd/rules.yaml`, project rules in `.shepherd/rules.yaml`. Project rules are additive (can add deny/allow, cannot override global deny).

## Data Model (Core Entities)

```
tasks: id, title, prompt, agent_id, repo_path, branch, isolation_mode,
       status (queued|running|input|review|error|done), created_at, updated_at

sessions: id, task_id, pty_pid, terminal_log_path, started_at, ended_at

permissions: id, task_id, tool_name, tool_args, decision (auto|approved|denied),
             rule_matched, decided_at

diffs: id, task_id, file_path, before_hash, after_hash, created_at

profiles: id, name, config_json, is_default

gate_results: id, task_id, gate_name, passed, output, ran_at
```

## WebSocket Events (Core)

```
server → client:
  task:created, task:updated, task:deleted
  terminal:output {task_id, data}
  permission:requested {task_id, tool, args}
  permission:resolved {task_id, decision}
  gate:result {task_id, gate, passed}
  notification {type, title, body}

client → server:
  task:create {title, agent, repo, isolation}
  task:approve {task_id}
  task:approve_all
  task:cancel {task_id}
  terminal:input {task_id, data}
  terminal:resize {task_id, cols, rows}
```

## V2 Features (Post-Launch)

- **Best-of-N generation** — run same task on N agents, compare outputs
- **Smart model routing** — auto-route tasks to optimal model based on complexity
- **Mobile monitoring** — QR code scan, push notifications, approve from phone
- **Team features** — shared dashboard, multi-user approval workflows
- **Browser UI** — same backend, web client for remote access
- **Adapter registry** — public registry for community agent adapters
