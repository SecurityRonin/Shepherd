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

Use the **iTerm2 WebSocket API protocol implemented natively in Rust** via `prost` (protobuf) and `tokio-tungstenite` (WebSocket). No Python sidecar. The iTerm2 API listens on a Unix socket at `~/Library/Application Support/iTerm2/iterm2-sock` and uses a protobuf schema defined in `api.proto` from the iTerm2 source tree.

This approach was chosen over a Python sidecar (cleaner packaging, no external runtime dependency) and over PTY file descriptor hijacking (blocked by macOS SIP).

---

## Architecture

### New module: `crates/shepherd-core/src/iterm2/`

```
iterm2/
  mod.rs        — public API: Iterm2Client, AdoptedSession, scanner
  client.rs     — WebSocket connection + protobuf send/recv
  session.rs    — AdoptedSession: iTerm2 session wrapped in PtySession interface
  scanner.rs    — periodic poller; emits AdoptionCandidate events
```

### Build-time proto compilation

`crates/shepherd-core/build.rs` fetches `api.proto` from the iTerm2 repository at a pinned commit SHA and invokes `prost_build` to generate Rust types. The pinned SHA ensures reproducible builds despite upstream changes.

### New server component: `Iterm2Scanner`

Spawned alongside `PtyManager` at server startup. Runs a 5-second poll loop. On each tick:

1. Sends `ListSessionsRequest` to the iTerm2 socket
2. For each session, fetches the `jobName` variable
3. If `jobName` contains `"claude"` and the session is not already adopted, emits an `AdoptionCandidate`
4. Server creates a task and an `AdoptedSession` immediately (no user confirmation required)

If the iTerm2 socket is absent (iTerm2 not running), the scanner logs a warning and retries next tick — the server never crashes.

---

## Data Flow

```
iTerm2 tab (claude process)
  │
  ├─► NOTIFY_ON_SCREEN_UPDATE ──► AdoptedSession ──► TerminalOutput event ──► WebSocket ──► Frontend xterm.js
  │
  └─◄ InjectRequest ◄── AdoptedSession ◄── TerminalInput / TaskApprove ◄── WebSocket ◄── Frontend
```

The `AdoptedSession` struct implements the same `write_to` / `kill` / `resize` interface as the existing PTY sessions. The rest of the server (WebSocket handler, gate detector, observability) is unchanged.

---

## Session Management

### Identity

A session is identified by its `cwd` (working directory), fetched via the iTerm2 `path` variable. Claude Code stores session state under `~/.claude/projects/<encoded-cwd>/`. Shepherd reads this directory to enumerate available session IDs for the resume/fresh-start UI.

### Actions

| Action | Mechanism |
|--------|-----------|
| **Auto-adopt** | Scanner detects `claude` in tab → creates task automatically |
| **Resume** | Send SIGINT to current process, relaunch in same tab with `claude --resume <session-id>` |
| **Start fresh** | Send SIGINT, relaunch without `--resume` — new session, no prior context |
| **Kill** | `InjectRequest` sends `\x03\x03`, or `SendSignalRequest` with SIGTERM |

### Gate enforcement

`NOTIFY_ON_SCREEN_UPDATE` provides rendered screen text. The existing gate detector runs against this text, matching patterns like `Allow bash tool? (y/n)`. Auto-approve and manual approval both work via `InjectRequest("y\n")` — identical to spawned sessions.

---

## API Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| `POST` | `/api/sessions/:id/resume` | Kill current process, relaunch with `--resume <session-id>` |
| `POST` | `/api/sessions/:id/fresh` | Kill current process, relaunch without `--resume` |
| `GET` | `/api/sessions/:id/claude-sessions` | List available Claude session IDs in `~/.claude/projects/<cwd>/` |

Existing endpoints (`POST /api/tasks`, WebSocket events) are unchanged.

---

## Frontend Changes

**Minimal.** Adopted sessions appear as ordinary task cards with two additions:

1. **`iTerm2` source badge** — a small pill on the task card distinguishing adopted sessions from Shepherd-spawned ones.
2. **Session picker in task detail** — dropdown of available `~/.claude/projects/` session IDs with `Resume` and `Start Fresh` buttons. Replaces the missing "prompt" field.

No new views, no routing changes. The existing `FocusView` terminal panel works unchanged since `AdoptedSession` feeds the same `TerminalOutput` event stream.

---

## Error Handling

| Scenario | Handling |
|----------|----------|
| iTerm2 not running / socket absent | Scanner skips silently, retries next poll |
| Proto schema drift (API version mismatch) | `prost` decode errors caught per-session; warning logged, session skipped |
| Claude process exits unexpectedly | `NOTIFY_ON_SESSION_ENDED` → task status set to `"done"` |
| User closes iTerm2 tab manually | Same notification; task marked done, not deleted |
| Duplicate adoption (scanner race) | Dedup by iTerm2 session ID in `HashSet<String>` |
| `~/.claude/projects/` missing | Resume endpoint returns 404; UI disables Resume button |
| Inject to closed session | API error swallowed silently |

---

## Testing

- **Unit tests** — `scanner.rs` and `session.rs` tested against a mock `Iterm2Client` replaying canned protobuf responses. No real iTerm2 required; runs in CI.
- **Integration tests** — gated behind `#[cfg(feature = "iterm2-integration")]`; skipped in CI, run manually with iTerm2 present.
- **Gate detection** — existing gate unit tests cover pattern matching; no new tests needed.
- **Proto build** — pinned commit SHA in `build.rs` ensures reproducible CI builds.

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `prost` + `prost-build` | Compile `api.proto` → Rust types |
| `tokio-tungstenite` | Async WebSocket client for the iTerm2 Unix socket |
| `tokio-util` | Codec framing for length-delimited protobuf messages |

All are pure Rust, no system dependencies beyond the existing `tokio` runtime.
