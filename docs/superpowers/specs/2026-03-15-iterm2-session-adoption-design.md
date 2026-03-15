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

The WebSocket handshake request is constructed with `http::Request`:

```rust
let req = http::Request::builder()
    .uri("ws://localhost/")
    .header("x-iterm2-cookie", &auth.cookie)
    .header("x-iterm2-key", &auth.key)
    .body(())?;
tokio_tungstenite::client_async_with_config(req, stream, None).await?
```

The `uri` is a placeholder (ignored for Unix socket transport); the `x-iterm2-cookie` and `x-iterm2-key` headers carry the per-process credentials required by the iTerm2 API.

---

## Authentication

The iTerm2 WebSocket API requires:

1. **Python API enabled** in iTerm2 Preferences → General → Magic → Enable Python API. If disabled, the socket exists but connections are immediately closed — handled as a distinct error from "socket absent."

2. **`x-iterm2-cookie` and `x-iterm2-key` WebSocket headers** — these are per-iTerm2-process values only available as `ITERM2_COOKIE` / `ITERM2_KEY` environment variables inside processes launched by iTerm2's script runner.

### Credential acquisition: AutoLaunch bridge script

Shepherd ships a minimal Python script installed at:
`~/Library/Application Support/iTerm2/Scripts/AutoLaunch/shepherd-bridge.py`

This script runs automatically when iTerm2 starts. iTerm2 injects `ITERM2_COOKIE` and `ITERM2_KEY` into the script's process environment; the script reads these values (`os.environ["ITERM2_COOKIE"]`, `os.environ["ITERM2_KEY"]`) and writes them to `~/.shepherd/iterm2-auth.json` (permissions: `0600`). Its **only job is credential forwarding** — it does not subscribe to iTerm2 events or forward any data. All event subscriptions are made directly by the Rust server.

```json
{ "cookie": "...", "key": "..." }
```

The Rust server (`iterm2/auth.rs`):
- Reads this file at startup
- Watches it with `notify` crate (`RecommendedWatcher` on macOS uses FSEvents callbacks). Because the callback is non-async, events are forwarded over a `std::sync::mpsc::channel`; an async task (`tokio::task::spawn_blocking` or a dedicated thread) reads that channel and sends change notifications into a `tokio::sync::mpsc::Sender` for consumption by the async reconnect handler. This avoids blocking the tokio executor.
- On file change: drops current connection, reads new credentials, reconnects
- If file absent: disables the iterm2 module and shows setup instructions in the UI

---

## Architecture

### New module: `crates/shepherd-core/src/iterm2/`

```
iterm2/
  mod.rs        — public API surface
  client.rs     — UnixStream WebSocket + protobuf framing (one message per binary frame)
  session.rs    — AdoptedSession: one iTerm2 session under Shepherd management
  scanner.rs    — periodic poll: discovers claude sessions, emits AdoptionCandidate
  auth.rs       — reads + watches ~/.shepherd/iterm2-auth.json
```

### Vendored proto

`crates/shepherd-core/proto/iterm2-api.proto` — vendored from iTerm2 at a pinned release tag. `build.rs` compiles it with `prost_build`. Generated types are included in `client.rs` via:

```rust
include!(concat!(env!("OUT_DIR"), "/iterm2.rs"));
```

Updated manually when the iTerm2 API changes.

### Scanner poll loop (5 s interval)

1. `ListSessionsRequest` → `ListSessionsResponse { repeated Window windows }`. Each `Window` has `repeated Tab tabs`; each `Tab` has a `SplitTreeNode root` (recursive split-pane tree). The scanner walks `windows → tabs → SplitTreeNode` recursively, collecting every `SessionSummary` leaf (fields used: `unique_identifier`, `title`).
2. For each session not already adopted (dedup by `iterm2_session_id` in `HashSet<String>`):
   - `VariableRequest { session: <unique_identifier>, name: "jobName" }` — **must set `session` field** to scope the query to this session; omitting it queries the focused session
   - If `jobName` contains `"claude"`: `VariableRequest { session: <unique_identifier>, name: "path" }` for CWD
   - Emit `AdoptionCandidate { iterm2_session_id, cwd }` (no `pid` — not available without extra round-trip)
   - Send `NotificationRequest { session: <unique_identifier>, subscribe: true, notification_type: NOTIFY_ON_SCREEN_UPDATE }` to begin receiving screen updates for this session
3. On first adoption, send a **single global** `NotificationRequest { subscribe: true, notification_type: NOTIFY_ON_TERMINATE_SESSION }` (omit `session` field — this notification type is not session-scoped). The handler filters incoming termination events by `session_id` in the notification payload to identify which adopted session exited.

---

## Data Flow

```
iTerm2 tab (claude process)
  │
  ├─► NOTIFY_ON_SCREEN_UPDATE (session_id only)
  │      └─► 50 ms trailing debounce ──► GetBufferRequest(lines: last 200, from scrollback tail)
  │                └─► cell flattening (hard_eol-aware) ──► gate detector + TerminalOutput event ──► WebSocket ──► frontend
  │
  ├─► NOTIFY_ON_TERMINATE_SESSION ──► task status → "done" / resume trigger
  │
  └─◄ SendTextRequest(text) ◄── AdoptedSession ◄── TerminalInput / TaskApprove ◄── WebSocket ◄── frontend
```

### Protobuf framing over WebSocket

Each `ClientOriginatedMessage` / `ServerOriginatedMessage` maps to exactly one WebSocket **binary frame**. The protobuf payload is the entire frame body — no additional length-delimiter prefix is added. The send path is: `prost::Message::encode_to_vec()` → `tungstenite::Message::Binary(bytes)`. The receive path is: `Message::Binary(bytes)` → `prost::Message::decode()`. No `tokio-util` codec is involved.

### Screen update debounce

`NOTIFY_ON_SCREEN_UPDATE` fires per cell update — potentially hundreds per second during active output. The `AdoptedSession` runs a **50 ms trailing debounce**: notifications accumulate in a flag; when no notification has arrived for 50 ms, one `GetBufferRequest` is issued. This keeps `GetBufferRequest` calls bounded to ~20/s maximum during heavy output.

### GetBufferResponse → plain text

`GetBufferResponse` returns structured per-character cell data (`LineContents` per line, each containing `Cell` items). Shepherd flattens it to plain text using a `hard_eol`-aware algorithm:

1. For each `LineContents` in the response:
   a. Concatenate the `string_value` field of every `Cell` in that line to form the line string.
   b. If `line_contents.hard_eol` is `true`, append `"\n"` after the line string.
   c. If `hard_eol` is `false` (soft-wrap continuation), append nothing — the line continues without a newline separator.
2. Concatenate all resulting strings.

This correctly preserves logical line boundaries without inserting spurious newlines at soft-wrap points.

`GetBufferRequest` requests the last 200 lines from the tail of the scrollback buffer. Set `session: <unique_identifier>` and use the `trailing_lines: 200` field (or equivalent — confirm against vendored proto field name; the semantics are "last N lines of the buffer"). If `GetBufferResponse` carries a non-OK status, the update is silently skipped and the next debounce cycle is awaited.

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
| Python API disabled (socket exists, WebSocket upgrade fails / connection closed immediately post-connect) | Distinct warning with "Enable Python API in iTerm2 Preferences" instruction |
| Auth file absent | iterm2 module disabled; setup instruction shown in UI |
| iTerm2 restarts (cookie/key rotated) | `notify` watcher detects file change → reconnect with new credentials |
| Proto decode error | Per-session warning logged, session skipped |
| Process exits unexpectedly | `NOTIFY_ON_TERMINATE_SESSION` → task status `"done"` |
| User closes tab | Same notification; task marked done, not deleted |
| Duplicate adoption | Dedup by `iterm2_session_id` in `HashSet<String>` in scanner |
| `~/.claude/projects/` missing | `/api/sessions/:id/claude-sessions` returns `[]`; UI disables Resume |
| `path` variable empty (no shell integration) | Falls back to `title`, then `"unknown"` |
| Relaunch process timeout | 5 s wait, then `CloseRequest(force: true)` before relaunch |
| Screen update flood | 50 ms debounce limits `GetBufferRequest` calls to ~20/s |
| `GetBufferResponse` non-OK status | Silently skip update; await next debounce cycle |

---

## Testing

- **Unit tests** — mock `Iterm2Client` replaying canned protobuf responses. Critical paths covered:
  - Discovery: `ListSessionsRequest` → `VariableRequest` round-trips → `AdoptionCandidate` emitted
  - Output: `NOTIFY_ON_SCREEN_UPDATE` → debounce → `GetBufferRequest` → cell flattening (hard_eol-aware) → gate pattern match
  - Input: `SendTextRequest` called with correct bytes for `TerminalInput` and `TaskApprove`
  - Exit: `NOTIFY_ON_TERMINATE_SESSION` → task status `"done"`
  - Resume: Ctrl-C → wait for terminate → relaunch command sent to shell
  - Auth rotation: file change → reconnect with new credentials
  - Debounce: burst of 100 notifications → single `GetBufferRequest` issued
  - Cell flattening: soft-wrap lines joined without `\n`; hard_eol lines terminated with `\n`
- **Integration tests** — gated behind `#[cfg(feature = "iterm2-integration")]`; skipped in CI
- **Proto build** — vendored file; no network dependency in CI

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `prost` + `prost-build` | Compile vendored `api.proto` → Rust types; generated types included via `include!(concat!(env!("OUT_DIR"), "/iterm2.rs"))` |
| `tokio-tungstenite` | Async WebSocket over `tokio::net::UnixStream` via `client_async_with_config` |
| `tokio` | Features required: `net`, `time`, `fs`, `sync` |
| `notify` | File-system watcher for `~/.shepherd/iterm2-auth.json` credential rotation |

**System requirement:** iTerm2 3.3+ with Python API enabled. iTerm2 shell integration recommended for accurate CWD detection but not required.
