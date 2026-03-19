# Shepherd v1.0 — Full Product Design Spec

**Date**: 2026-03-19
**Status**: Approved
**Approach**: Vertical Slice (Phase 1 → 4)

## Overview

Shepherd is a cross-platform desktop application for managing coding agents (Claude Code, Aider, Opencode, Codex, Gemini CLI). It provides a unified GUI and CLI for task orchestration, agent dispatch, permission management, and observability.

This spec covers the work needed to take Shepherd from an engineering prototype to a shippable end-user product.

## Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Target scope | Full product | All five gaps addressed: server, orchestration, install, GUI, cloud |
| Task dispatch model | Dual mode — supervised + YOLO per task | Leverages existing YoloEngine and RuleSet; gives user flexibility |
| Server lifecycle | Tauri embeds server; CLI auto-spawns daemon | Cleanest UX for GUI users, HTTP API available for CLI/integrations |
| Platforms | macOS + Linux + Windows | Tauri supports all three; full developer audience |
| Cloud model | Local-first with cloud sync | Works offline with own API keys, better with cloud (sync, credits, teams) |
| First-run UX | Straight to dashboard | Auto-detect agents in background, dismissible setup banner, zero friction |
| Cloud architecture | Local proxy to shepherd-pro | Clean arch: frontend talks to one backend, CloudClient is a typed SDK, local server handles offline queue + credentials |
| Installation | Platform-native package managers | Homebrew (macOS), apt repo (Linux), winget (Windows), AppImage fallback, cargo install for source builders |
| CLI packaging | Single binary does everything | `shep` launches GUI, runs CLI commands, or runs headless server — one install, all capabilities |
| Monetization | In-app tiered model | Free local features, Pro for cloud sync/credits/team — nudges happen inside the app users already have |

## Architecture

### System Overview

```
┌─────────────────────────────────────────────────┐
│                  Tauri Desktop App               │
│  ┌──────────────┐  ┌────────────────────────┐   │
│  │ React UI     │  │ Embedded Axum Server   │   │
│  │ (46 components)│ │ (HTTP + WebSocket)     │   │
│  │ Zustand store│  │                        │   │
│  └──────┬───────┘  │  ┌──────────────────┐  │   │
│         │          │  │ TaskDispatcher    │  │   │
│   Tauri Commands   │  │ PtyManager       │  │   │
│   + WebSocket      │  │ YoloEngine       │  │   │
│         │          │  │ ContextOrchestrator│ │   │
│         └──────────┤  │ CloudClient ──────┼──┼───┼──→ shepherd-pro
│                    │  └──────────────────┘  │   │     (Next.js cloud)
│                    │  SQLite DB             │   │
│                    └────────────────────────┘   │
└─────────────────────────────────────────────────┘
         ▲
         │ HTTP (auto-spawned daemon)
         │
    ┌────┴─────┐
    │ shep CLI │
    └──────────┘
```

### Single Binary Model

`shep` is one binary that does everything:
- `shep` (no args) → launches Tauri desktop GUI with embedded server
- `shep new "task"` → CLI mode, auto-starts daemon if not running
- `shep status` → CLI mode
- `shep --headless` → server-only daemon mode (SSH/remote use)
- `shep stop` → kills daemon

## Section 1: Embedded Server & CLI Daemon

### Server Embedding

The Axum server logic lives in a reusable function in `shepherd-server/src/lib.rs`:

```rust
pub async fn start_server(config: ShepherdConfig) -> Result<(SocketAddr, JoinHandle<()>)>
```

**Tauri app**: calls `start_server()` in a background Tokio task during `tauri::Builder::setup()`. Server runs in-process, shares `AppState`. Tauri commands access `AppState` directly (no HTTP roundtrip for CRUD operations). WebSocket used only for real-time streaming (terminal output, events).

**CLI daemon**: when `shep` runs any command, it checks if a server is reachable at the configured port (default 7532). If not, spawns `shepherd-server` as a background daemon (`daemonize` on Unix, `CREATE_NO_WINDOW` on Windows).

### Server Discovery

Both Tauri and CLI write `~/.shepherd/server.json`:
```json
{ "pid": 12345, "port": 7532, "started_at": "2026-03-19T10:00:00Z" }
```

This prevents port conflicts and lets multiple tools find the running instance.

### Graceful Shutdown

- Server listens for SIGTERM
- Tauri sends SIGTERM on app quit
- `shep stop` sends SIGTERM
- Configurable idle timeout (default 30 min) for daemon mode — auto-exits if no clients connected

## Section 2: Agent Orchestration — Task Dispatch Loop

### TaskDispatcher

A new component in `shepherd-core` that runs as a background Tokio task inside the server:

```
User creates task (GUI/CLI/API)
  → TaskDispatcher picks it up
    → Resolves adapter (from task.agent_id or default_agent config)
    → Acquires file lock (coordination.rs)
    → Spawns PTY session via PtyManager
    → Constructs agent command with task context
    → Streams PTY output over WebSocket
    → Monitors for permission requests
    → Routes approvals back to PTY stdin
    → On completion: updates task status, releases lock
```

### Task Lifecycle States

```
pending → dispatching → running → [waiting_approval] → running → completed/failed
```

### SessionMonitor

Parses PTY output in real-time for permission request patterns. Each adapter defines its own regex patterns in the `.toml` config.

- **Supervised mode**: emits `PermissionRequired` WebSocket event, pauses agent, waits for `TaskApprove`/`TaskReject` from GUI/CLI
- **YOLO mode**: checks `YoloEngine` rule set. Allow-matched actions auto-approve to PTY stdin. Deny-matched actions pause and escalate.

### Context Injection

When spawning an agent, `ContextOrchestrator` gathers relevant files, symbols, and imports. This context is injected alongside the task description. Each adapter config defines how context gets injected (e.g., Claude Code via file paths, Aider via `/add` commands).

### Concurrency

- `max_agents` config (default 4) limits parallel PTY sessions
- Dispatcher queues excess tasks
- File locking (coordination.rs) prevents two agents from editing the same file

## Section 3: Tauri Desktop App Integration

### App Startup Sequence

1. Tauri `setup()` hook loads config from `~/.shepherd/config.toml`
2. Checks `~/.shepherd/server.json` — if server already running, connects to it
3. If no server found, starts embedded Axum server on background Tokio runtime
4. Writes `server.json` with PID and port
5. Auto-detects installed agents by checking PATH for known binaries
6. Opens main window → kanban dashboard with dismissible setup banner if first run

### Tauri Commands (Rust → JS Bridge)

- `create_task`, `list_tasks`, `update_task` — direct DB access via shared `AppState`
- `approve_task`, `reject_task` — sends approval to PTY stdin via `SessionMonitor`
- `get_terminal_output` — subscribes to PTY output stream for focus view
- `terminal_input` — sends user keystrokes to PTY stdin
- `get_config`, `update_config` — config read/write

### WebSocket (Real-time Streaming Only)

- Terminal output frames
- Task status change events
- Permission request notifications
- Agent status updates (idle/working/error)

### Frontend Store Wiring

The existing Zustand store has 4 slices:
- `TasksSlice` → Tauri `invoke` calls for CRUD, WebSocket for live updates
- `SessionsSlice` → WebSocket terminal stream
- `UiSlice` → local state (unchanged)
- `ObservabilitySlice` → periodic API poll for metrics

### Platform-Specific Builds

- macOS: `.app` bundle, code signing via Apple Developer ID
- Linux: `.AppImage` (universal) + `.deb` (Debian/Ubuntu)
- Windows: `.msi` installer via WiX, ConPTY for terminal sessions

## Section 4: Cloud Integration with shepherd-pro

### Architecture

Clean proxy pattern — frontend talks to one backend:

```
Frontend → Local Axum (proxy) → CloudClient (typed SDK) → shepherd-pro (business logic)
```

- Frontend knows one API surface (local server)
- CloudClient is a typed Rust SDK for the cloud API (not duplicated logic)
- Local proxy adds offline queuing, credential injection from OS keychain
- Extra hop is localhost-only (negligible latency)

### Authentication Flow

1. User clicks "Connect Cloud" in settings → opens browser to `shepherd.pro/auth`
2. OAuth flow (GitHub/Google) in shepherd-pro, issues JWT
3. Callback redirects to `shepherd://auth?token=<jwt>` (Tauri deep link)
4. Token stored in OS keychain via `keyring` crate (macOS Keychain, Linux Secret Service, Windows Credential Manager)
5. All subsequent `CloudClient` calls include JWT in `Authorization` header

### Sync

- Bidirectional: tasks, session metadata, adapter configs
- Conflict resolution: timestamps + last-writer-wins for simple fields
- Triggers: on task state change, on app focus, manual "sync now", periodic (every 5 min)
- Offline-resilient: queues changes locally, replays on reconnect

### Feature Matrix by Connectivity

| Feature | Offline (own API keys) | Cloud (shepherd-pro) |
|---|---|---|
| Task orchestration | Full | Full + synced |
| Logo generation | OpenAI key in config | Cloud credits |
| Name generation | OpenAI key in config | Cloud credits + RDAP checks |
| Templates | Local `.toml` files | Shared template gallery |
| Notifications | Desktop only (Tauri native) | Push + desktop |
| Observability | Local SQLite metrics | Dashboard + history |
| Team features | N/A | Shared tasks, permissions |

### API Key Management (Offline Mode)

```toml
[llm]
provider = "openai"
api_key_env = "OPENAI_API_KEY"  # reads from env var, never stored in config
```

### Credits System

Cloud AI features deduct credits. Free tier gets N credits/month. Paid plans get more. `CreditBalance` component wired to `CloudClient::get_credits()`.

## Section 5: Installation & Distribution

### Install Priority by Platform

**macOS** (primary: Homebrew):
```bash
brew install h4x0r/tap/shepherd
```

**Linux** (primary: apt):
```bash
# One-time repo setup
curl -fsSL https://apt.shepherd.pro/key.gpg | sudo gpg --dearmor -o /usr/share/keyrings/shepherd.gpg
echo "deb [signed-by=/usr/share/keyrings/shepherd.gpg] https://apt.shepherd.pro stable main" | sudo tee /etc/apt/sources.list.d/shepherd.list
sudo apt update && sudo apt install shepherd
```

Fallback: `.AppImage` download from GitHub Releases.

**Windows** (primary: winget):
```powershell
winget install shepherd
```

Fallback: `.msi` download from GitHub Releases.

**Source build** (all platforms):
```bash
cargo install shepherd
```

### CI Pipeline (GitHub Actions)

```
on push to main:
  - cargo test --workspace
  - cargo clippy
  - cargo tarpaulin (coverage gate: 100%)

on tag v*:
  - matrix build: [macos-latest, ubuntu-latest, windows-latest]
  - cargo tauri build for each platform
  - code sign (macOS: apple-certificate, Windows: signtool)
  - upload artifacts to GitHub Release
  - update Homebrew tap formula
  - publish to crates.io
```

### Auto-Update

Tauri updater plugin checks for new versions on app launch (configurable frequency). Non-intrusive notification: "Update available: v1.2.0" — user clicks to update in-place.

## Section 6: Implementation Phasing

### Phase 1 — Core Loop (Vertical Slice)

- Embed Axum server in Tauri app startup
- CLI daemon auto-spawn with `server.json` discovery
- `TaskDispatcher`: pending task → adapter resolution → PTY spawn → stream output
- Wire one agent end-to-end: Claude Code
- Connect React kanban → real task CRUD via Tauri commands
- Connect Focus view → real PTY terminal stream via WebSocket
- Permission prompt UI → approval/rejection flow to PTY stdin
- YOLO engine wiring (rules from config, per-task toggle)

### Phase 2 — Full Agent Support

- All 5 adapters wired and tested (Aider, Opencode, Codex, Gemini CLI)
- SessionMonitor permission patterns per adapter
- Context injection via ContextOrchestrator per adapter's format
- File-lock coordination for multi-agent parallel tasks
- Quality gates UI connected
- Observability dashboard wired to real metrics

### Phase 3 — Cloud Integration

- Auth flow: OAuth → JWT → OS keychain → Tauri deep link callback
- Sync: bidirectional task/session sync with offline queue
- Credits: balance display, cloud AI features
- Templates: gallery fetch, apply to new projects
- Notifications: push via shepherd-pro, desktop via Tauri native

### Phase 4 — Distribution & Polish

- CI matrix: macOS + Linux + Windows builds via GitHub Actions
- Code signing: Apple Developer ID, Windows signtool
- Homebrew tap with bottled formula
- Apt repository at apt.shepherd.pro
- Winget manifest
- AppImage and .msi on GitHub Releases
- Tauri auto-updater connected to shepherd.pro/api/releases
- First-run: auto-detect agents, dismissible banner
- cargo install shepherd as source-build fallback
