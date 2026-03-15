# iTerm2 Session Adoption — Design Spec

**Date:** 2026-03-15
**Status:** Approved
**Scope:** Adopt existing Claude Code sessions running in iTerm2 tabs as first-class Shepherd tasks with full bidirectional I/O, gate enforcement, and session management.

---

## Problem

Shepherd currently only manages Claude Code sessions it spawns itself via its internal PTY layer. Users who start `claude` manually in an iTerm2 tab get none of Shepherd's benefits — no task tracking, no gate enforcement, no observability. There is no way to bring an existing session under Shepherd management.

---

## Goals

- Auto-discover iTerm2 tabs running `claude` and adopt them as first-class Shepherd tasks
- Mirror terminal output in the Shepherd board (dual-view: tab stays live in iTerm2)
- Forward input from the Shepherd terminal panel back into the iTerm2 tab
- Enforce gates (permission prompts) on adopted sessions identically to spawned ones
- Support per-session management: resume via `claude --resume <id>` or start fresh

## Non-Goals

- Supporting terminal emulators other than iTerm2
- Taking exclusive control of a tab (closing or locking it in iTerm2)
- Capturing output from sessions started before Shepherd was running

---

## Approach

Use the **iTerm2 WebSocket API protocol implemented natively in Rust** via `prost` (protobuf) and `tokio-tungstenite` (WebSocket). The iTerm2 API socket is discovered by globbing `~/Library/Application Support/iTerm2/iterm2-daemon-*.socket` (name is versioned per iTerm2 process instance). Proto is vendored in the repository at a pinned iTerm2 release; `build.rs` compiles it with `prost_build` — no network access at build time.

### WebSocket over Unix domain socket

`tokio-tungstenite` does not natively connect over Unix sockets. The client opens the socket with `tokio::net::UnixStream::connect()`, then calls `tokio_tungstenite::client_async_with_config()` passing the stream directly. This is the documented pattern for non-TCP transports.

---

## Authentication

The iTerm2 WebSocket API requires:

1. **Python API enabled** in iTerm2 Preferences → General → Magic → Enable Python API. If disabled, the socket exists but connections are immediately closed — handled as a distinct error from "socket absent."

2. **`cookie` and `key` WebSocket headers** — these are per-iTerm2-process values only available as `ITERM2_COOKIE` / `ITERM2_KEY` environment variables inside processes launched by iTerm2's script runner.

### Credential acquisition: AutoLaunch bridge script

Shepherd ships a minimal Python script installed at:
`~/Library/Application Support/iTerm2/Scripts/AutoLaunch/shepherd-bridge.py`

This script runs automatically when iTerm2 starts, has the cookie/key env vars, and writes them to `~/.shepherd/iterm2-auth.json` (permissions: `0600`). Its **only job is credential forwarding** — it does not subscribe to iTerm2 events or forward any data. All event subscriptions are made directly by the Rust server.

```json
{ "cookie": "...", "key": "..." }
```

The Rust server (`iterm2/auth.rs`):
- Reads this file at startup
- Watches it with `notify` crate for changes (handles iTerm2 restarts, which rotate cookie/key and cause the bridge to rewrite the file)
- On file change: drops current connection, reads new credentials, reconnects
- If file absent: disables the iterm2 module and shows setup instructions in the UI

---

## Architecture

### New module: `crates/shepherd-core/src/iterm2/`

```
iterm2/
  mod.rs        — public API surface
  client.rs     — UnixStream WebSocket + protobuf framing (length-delimited)
  session.rs    — AdoptedSession: one iTerm2 session under Shepherd management
  scanner.rs    — periodic poll: discovers claude sessions, emits AdoptionCandidate
  auth.rs       — reads + watches ~/.shepherd/iterm2-auth.json
```

### Vendored proto

`crates/shepherd-core/proto/iterm2-api.proto` — vendored from iTerm2 at a pinned release tag. `build.rs` compiles it with `prost_build`. Updated manually when the iTerm2 API changes.

### Scanner poll loop (5 s interval)

1. `ListSessionsRequest` → `SessionSummary` list (fields: `unique_identifier`, `title`, `frame`, `grid_size`)
2. For each session not already adopted:
   - `VariableRequest(name: "jobName")` — one round-trip per unadopted session
   - If `jobName` contains `"claude"`: `VariableRequest(name: "path")` for CWD
   - Emit `AdoptionCandidate { iterm2_session_id, cwd }` (no `pid` — not available without extra round-trip)
3. For each newly adopted session: subscribe to `NOTIFY_ON_TERMINATE_SESSION` for immediate exit detection

---

## Data Flow

```
iTerm2 tab (claude process)
  │
  ├─► NOTIFY_ON_SCREEN_UPDATE (session_id only)
  │      └─► 50 ms trailing debounce ──► GetBufferRequest(lines: last 200)
  │                └─► flatten cells to text ──► gate detector + TerminalOutput event ──► WebSocket ──► frontend
  │
  ├─► NOTIFY_ON_TERMINATE_SESSION ──► task status → "done" / resume trigger
  │
  └─◄ SendTextRequest(text) ◄── AdoptedSession ◄── TerminalInput / TaskApprove ◄── WebSocket ◄── frontend
```

### Screen update debounce

`NOTIFY_ON_SCREEN_UPDATE` fires per cell update — potentially hundreds per second during active output. The `AdoptedSession` runs a **50 ms trailing debounce**: notifications accumulate in a flag; when no notification has arrived for 50 ms, one `GetBufferRequest` is issued. This keeps `GetBufferRequest` calls bounded to ~20/s maximum during heavy output.

### GetBufferResponse → plain text

`GetBufferResponse` returns structured per-character cell data (character, color, attributes). Shepherd flattens it to plain text by iterating each line's cells, extracting the `string_value` of each `Cell`, joining cells into a line string, and joining lines with `\n`. This plain text is what the gate detector and the `TerminalOutput` event both receive.

Only the last 200 lines are requested per `GetBufferRequest` (sufficient for gate detection and terminal display).

### Input

`SendTextRequest` sends text to the process's stdin. This is the correct API method. `InjectRequest` writes to the display buffer and is not used for input.

---

## Session Management

### Identity: CWD and Claude session ID

**CWD** comes from the `path` iTerm2 variable, which requires iTerm2 shell integration. Without it, `path` may be empty; Shepherd falls back to the session `title`, then `"unknown"`.

**Claude session ID** (for `--resume`) is discovered from `~/.claude/projects/<encoded-cwd>/` — Shepherd lists `.jsonl` files in that directory, sorted by modification time, and extracts the session ID from the filename or first-line metadata. This is the UUID passed to `claude --resume <id>`, distinct from the iTerm2 session ID.

### Actions

| Action | Mechanism |
|--------|-----------|
| **Auto-adopt** | Scanner finds `claude` in tab → task created immediately |
| **Resume** | `SendTextRequest("\x03")` to send Ctrl-C → wait for `NOTIFY_ON_TERMINATE_SESSION` → `SendTextRequest("claude --resume <claude-session-id>\n")` to shell prompt |
| **Start fresh** | Same, but send `SendTextRequest("claude\n")` without `--resume` |
| **Kill** | `CloseRequest(force: true)` |

**Relaunch mechanism:** After `NOTIFY_ON_TERMINATE_SESSION`, the iTerm2 tab returns to the shell prompt. Shepherd sends the relaunch command as a `SendTextRequest` to that shell — the shell executes `claude --resume <id>` naturally. No AppleScript, no new PTY, no new tab.

**Race prevention:** Shepherd never sends the relaunch command until `NOTIFY_ON_TERMINATE_SESSION` is received. A timeout of 5 s is applied; if the process hasn't terminated, `CloseRequest(force: true)` is used before relaunching.

### Gate enforcement

On each `NOTIFY_ON_SCREEN_UPDATE` → debounce → `GetBufferRequest` cycle, the plain-text result is passed to the existing gate detector. Permission patterns (`Allow bash tool? (y/n)`) are matched against this text. Auto-approve and manual approval send `SendTextRequest("y\n")` or `SendTextRequest("n\n")` respectively.

### Terminal resize

`AdoptedSession.resize(cols, rows)` sends a `SetSizeRequest` to iTerm2, which resizes the terminal grid. This satisfies the `resize()` interface the rest of the server expects.

---

## API Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| `POST` | `/api/sessions/:id/resume` | Ctrl-C, wait for exit, send `claude --resume <session-id>` to shell |
| `POST` | `/api/sessions/:id/fresh` | Ctrl-C, wait for exit, send `claude` to shell |
| `GET` | `/api/sessions/:id/claude-sessions` | List Claude Code session IDs from `~/.claude/projects/<cwd>/`, sorted newest-first, for the resume picker dropdown |

---

## Frontend Changes

**Minimal.** Adopted sessions appear as ordinary task cards with two additions:

1. **`iTerm2` source badge** — small pill on the task card.
2. **Session picker in task detail** — dropdown of Claude session IDs from `GET /api/sessions/:id/claude-sessions` with `Resume` and `Start Fresh` buttons (replaces the "prompt" field present on Shepherd-spawned tasks).
3. **Setup prompt** — if `~/.shepherd/iterm2-auth.json` is absent, a one-time callout in the UI explains how to install the bridge script.

---

## Error Handling

| Scenario | Handling |
|----------|----------|
| Socket absent (iTerm2 not running) | Silent retry each poll cycle |
| Python API disabled (socket present, connection refused) | Distinct warning with "Enable Python API in iTerm2 Preferences" instruction |
| Auth file absent | iterm2 module disabled; setup instruction shown in UI |
| iTerm2 restarts (cookie/key rotated) | `notify` watcher detects file change → reconnect with new credentials |
| Proto decode error | Per-session warning logged, session skipped |
| Process exits unexpectedly | `NOTIFY_ON_TERMINATE_SESSION` → task status `"done"` |
| User closes tab | Same notification; task marked done, not deleted |
| Duplicate adoption | Dedup by iTerm2 session ID in `HashSet<String>` in scanner |
| `~/.claude/projects/` missing | `/api/sessions/:id/claude-sessions` returns `[]`; UI disables Resume |
| `path` variable empty (no shell integration) | Falls back to `title`, then `"unknown"` |
| Relaunch process timeout | 5 s wait, then `CloseRequest(force: true)` before relaunch |
| Screen update flood | 50 ms debounce limits `GetBufferRequest` calls to ~20/s |

---

## Testing

- **Unit tests** — mock `Iterm2Client` replaying canned protobuf responses. Critical paths covered:
  - Discovery: `ListSessionsRequest` → `VariableRequest` round-trips → `AdoptionCandidate` emitted
  - Output: `NOTIFY_ON_SCREEN_UPDATE` → debounce → `GetBufferRequest` → cell flattening → gate pattern match
  - Input: `SendTextRequest` called with correct bytes for `TerminalInput` and `TaskApprove`
  - Exit: `NOTIFY_ON_TERMINATE_SESSION` → task status `"done"`
  - Resume: Ctrl-C → wait for terminate → relaunch command sent to shell
  - Auth rotation: file change → reconnect with new credentials
  - Debounce: burst of 100 notifications → single `GetBufferRequest` issued
- **Integration tests** — gated behind `#[cfg(feature = "iterm2-integration")]`; skipped in CI
- **Proto build** — vendored file; no network dependency in CI

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `prost` + `prost-build` | Compile vendored `api.proto` → Rust types |
| `tokio-tungstenite` | Async WebSocket over `tokio::net::UnixStream` via `client_async_with_config` |
| `tokio-util` | Length-delimited codec framing for protobuf messages |
| `notify` | File-system watcher for `~/.shepherd/iterm2-auth.json` credential rotation |

**System requirement:** iTerm2 3.3+ with Python API enabled. iTerm2 shell integration recommended for accurate CWD detection but not required.
