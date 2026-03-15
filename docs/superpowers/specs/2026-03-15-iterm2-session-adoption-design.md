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
- Capturing output from sessions started before Shepherd was running (only live output from adoption point onward)

---

## Approach

Use the **iTerm2 WebSocket API protocol implemented natively in Rust** via `prost` (protobuf) and `tokio-tungstenite` (WebSocket). No Python sidecar. The iTerm2 API listens on a Unix socket whose path matches `~/Library/Application Support/iTerm2/iterm2-daemon-*.socket` (versioned name; discovered by glob). Messages use a protobuf schema defined in `api.proto` from the iTerm2 source tree, vendored in the repository at a pinned version.

---

## API Authentication

The iTerm2 WebSocket API requires:

1. **Python API enabled in iTerm2 preferences** (Preferences → General → Magic → Enable Python API). If not enabled, the socket exists but connections are immediately refused — Shepherd detects this and shows a one-time actionable warning distinct from "iTerm2 not running."

2. **`cookie` and `key` headers** in the WebSocket handshake. These values are available as environment variables `ITERM2_COOKIE` and `ITERM2_KEY` only when a process is launched from within an iTerm2 Python script context. Since Shepherd is a standalone daemon, it cannot obtain these values from its own environment.

**Solution — companion launch script:** Shepherd ships a small shell script (`shepherd-iterm2-bridge`) that iTerm2 executes via its "AutoLaunch" script mechanism (`~/Library/Application Support/iTerm2/Scripts/AutoLaunch/shepherd-bridge.py`). This script:
- Has access to `ITERM2_COOKIE` and `ITERM2_KEY` automatically
- Writes them to a well-known file (`~/.shepherd/iterm2-auth.json`) that the Rust server reads on startup
- Subscribes to session events and forwards them to the Shepherd server via a local Unix socket or named pipe

The Rust server reads `~/.shepherd/iterm2-auth.json` to get the cookie/key pair and includes them as WebSocket headers in all connections to the iTerm2 socket. If the file is absent, the iterm2 module is disabled with a clear setup instruction in the Shepherd UI.

---

## Architecture

### New module: `crates/shepherd-core/src/iterm2/`

```
iterm2/
  mod.rs        — public API: Iterm2Client, AdoptedSession, scanner
  client.rs     — WebSocket connection + protobuf send/recv
  session.rs    — AdoptedSession: wraps one iTerm2 session ID
  scanner.rs    — periodic poller; emits AdoptionCandidate events
  auth.rs       — reads ~/.shepherd/iterm2-auth.json; provides cookie/key
```

### Vendored proto

`crates/shepherd-core/proto/iterm2-api.proto` — vendored copy of the iTerm2 `api.proto` at a specific iTerm2 release. Updated manually when the iTerm2 API changes. `build.rs` compiles it with `prost_build` — no network access required at build time.

### New server component: `Iterm2Scanner`

Spawned alongside `PtyManager` at server startup. Runs a 5-second poll loop:

1. Sends `ListSessionsRequest` — gets `SessionSummary` list (fields: `unique_identifier`, `frame`, `grid_size`, `title`)
2. For each session not already adopted, sends a `VariableRequest` for the `jobName` variable (one round-trip per unadopted session per poll)
3. If `jobName` contains `"claude"`, also fetches the `path` variable (requires iTerm2 shell integration; falls back to empty string if unavailable)
4. Emits `AdoptionCandidate { iterm2_session_id, cwd, pid }` for new candidates
5. Subscribes to `NOTIFY_ON_TERMINATE_SESSION` for each adopted session (immediate exit detection, no 5-second poll lag)

---

## Data Flow

```
iTerm2 tab (claude process)
  │
  ├─► NOTIFY_ON_SCREEN_UPDATE (session ID only)
  │       └─► GetBufferRequest ──► screen text ──► gate detector + TerminalOutput event ──► WebSocket ──► frontend
  │
  └─◄ SendTextRequest ◄── AdoptedSession ◄── TerminalInput / TaskApprove ◄── WebSocket ◄── frontend
```

**Key API corrections:**

- **Input**: `SendTextRequest` (sends keystrokes to the process's stdin). `InjectRequest` writes to the *display buffer* and must NOT be used for sending input to the running process.
- **Output**: `NOTIFY_ON_SCREEN_UPDATE` delivers only a session ID notification. A follow-up `GetBufferRequest` is issued to retrieve the actual screen text. This adds one round-trip per screen update.
- **Process exit**: `NOTIFY_ON_TERMINATE_SESSION` subscription provides immediate notification; no polling needed.

`AdoptedSession` implements the same `write_to` / `kill` / `resize` interface as existing PTY sessions so the rest of the server (WebSocket handler, gate detector, observability) is unchanged.

---

## Session Management

### Identity

A session is identified by its `cwd`, fetched via the iTerm2 `path` variable. **Note:** `path` is only accurate when iTerm2 shell integration is installed in the user's shell. Without it, `path` may be empty or stale. In this case, Shepherd falls back to the session's `title` string (which often contains the directory), and as a last resort leaves `cwd` as `"unknown"`.

Claude Code stores session state under `~/.claude/projects/<encoded-cwd>/`. Shepherd reads this directory to enumerate available session IDs for the resume/fresh-start UI.

### Actions

| Action | Mechanism |
|--------|-----------|
| **Auto-adopt** | Scanner detects `claude` in tab → creates task immediately |
| **Resume** | Send `SendTextRequest("\x03")`, subscribe to `NOTIFY_ON_TERMINATE_SESSION`, relaunch with `claude --resume <session-id>` only after termination notification |
| **Start fresh** | Same as resume but without `--resume` flag |
| **Kill** | `CloseRequest(force: true)` via the iTerm2 API, which cleanly closes the session |

**Resume race condition:** The server waits for `NOTIFY_ON_TERMINATE_SESSION` before issuing the relaunch command, preventing two `claude` processes competing for the same session file.

### Gate enforcement

`NOTIFY_ON_SCREEN_UPDATE` triggers a `GetBufferRequest`. The resulting screen text is passed to the existing gate detector, which matches patterns like `Allow bash tool? (y/n)`. Auto-approve and manual approval both use `SendTextRequest("y\n")`.

---

## API Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| `POST` | `/api/sessions/:id/resume` | Kill current process, wait for exit, relaunch with `--resume <session-id>` |
| `POST` | `/api/sessions/:id/fresh` | Kill current process, wait for exit, relaunch without `--resume` |
| `GET` | `/api/sessions/:id/claude-sessions` | List available Claude session IDs in `~/.claude/projects/<cwd>/` |

---

## Frontend Changes

**Minimal.** Adopted sessions appear as ordinary task cards with two additions:

1. **`iTerm2` source badge** — a small pill on the task card.
2. **Session picker in task detail** — dropdown of available `~/.claude/projects/` session IDs with `Resume` and `Start Fresh` buttons. Shown instead of the "prompt" field.

**Setup prompt:** If `~/.shepherd/iterm2-auth.json` is absent, the Cloud Settings (or a new Setup panel) shows a one-time instruction: "To enable iTerm2 adoption, install the Shepherd bridge script."

---

## Error Handling

| Scenario | Handling |
|----------|----------|
| iTerm2 not running (socket absent) | Scanner skips silently, retries next poll |
| Python API disabled (socket present, connection refused) | One-time warning with setup instruction shown in UI |
| `iterm2-auth.json` absent (no cookie/key) | iterm2 module disabled; setup instruction shown |
| Proto schema drift / decode error | Per-session warning logged, session skipped |
| Process exits unexpectedly | `NOTIFY_ON_TERMINATE_SESSION` → task status `"done"` |
| User closes tab manually | Same notification; task marked done, not deleted |
| Duplicate adoption (scanner race) | Dedup by iTerm2 session ID in `HashSet<String>` |
| `~/.claude/projects/` missing | Resume endpoint returns 404; UI disables Resume button |
| `path` variable empty (no shell integration) | Falls back to `title`, then `"unknown"` |
| Resume relaunch race | Wait for `NOTIFY_ON_TERMINATE_SESSION` before relaunching |

---

## Testing

- **Unit tests** — `scanner.rs`, `session.rs`, `auth.rs` tested against a mock `Iterm2Client` that replays canned protobuf responses. Covers: discovery flow, `VariableRequest` round-trips, screen-update → GetBuffer → gate detector pipeline, `SendTextRequest` for input, terminate notification → task-done transition, resume wait-for-exit sequence.
- **Integration tests** — gated behind `#[cfg(feature = "iterm2-integration")]`; skipped in CI, run manually with iTerm2 present.
- **Gate detection** — dedicated tests for the `NOTIFY_ON_SCREEN_UPDATE` → `GetBufferRequest` → gate pattern match path.
- **Proto build** — vendored `.proto` file; no network access needed in CI.

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `prost` + `prost-build` | Compile vendored `api.proto` → Rust types |
| `tokio-tungstenite` | Async WebSocket client for the iTerm2 Unix socket |
| `tokio-util` | Codec framing for length-delimited protobuf messages |

**System requirement:** iTerm2 3.3+ with Python API enabled. iTerm2 shell integration recommended (for accurate `path`/`cwd` detection) but not required.
