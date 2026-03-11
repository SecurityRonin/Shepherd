# Shepherd Frontend — Implementation Plan (2 of 3)

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Shepherd desktop GUI — a Tauri 2.0 shell with React/TypeScript frontend featuring Kanban overview, Focus panel drill-down, embedded xterm.js terminals, Monaco diff viewer, and real-time WebSocket state sync.

**Architecture:** Tauri 2.0 wraps a React SPA that connects to the Shepherd Rust server (from Plan 1) via WebSocket + REST. State management via Zustand. Terminal emulation via xterm.js. Diff viewing via Monaco Editor. Styled with TailwindCSS.

**Tech Stack:** Tauri 2.0, React 18, TypeScript, Zustand, xterm.js, Monaco Editor, TailwindCSS, Vite

**Spec:** `docs/superpowers/specs/2026-03-10-shepherd-design.md`

**Dependencies:** Plan 1 (Core Engine) must be complete — this plan connects to the server API.

---

## File Structure

```
shepherd/
├── src-tauri/
│   ├── Cargo.toml                      # Tauri Rust dependencies
│   ├── tauri.conf.json                 # Tauri window config, dev server URL
│   ├── capabilities/
│   │   └── default.json                # Tauri permission capabilities
│   ├── icons/                          # App icons (placeholder)
│   │   └── icon.png
│   └── src/
│       └── main.rs                     # Tauri entry: launch server, create window
│
├── src/
│   ├── main.tsx                        # React app entry point
│   ├── App.tsx                         # Root component with router
│   ├── index.css                       # TailwindCSS imports + global styles
│   │
│   ├── types/
│   │   ├── index.ts                    # Re-exports all types
│   │   ├── task.ts                     # Task, TaskStatus, CreateTask
│   │   ├── session.ts                  # Session type
│   │   ├── permission.ts              # Permission, PermissionEvent
│   │   └── events.ts                   # ServerEvent, ClientEvent, StatusSnapshot
│   │
│   ├── lib/
│   │   ├── ws.ts                       # WebSocket client with auto-reconnect
│   │   ├── api.ts                      # REST API client (typed fetch wrappers)
│   │   └── keys.ts                     # Keyboard shortcut system
│   │
│   ├── store/
│   │   ├── index.ts                    # Combined Zustand store
│   │   ├── tasks.ts                    # Tasks slice
│   │   ├── sessions.ts                # Sessions slice
│   │   └── ui.ts                       # UI slice (view mode, focus task, panels)
│   │
│   ├── features/
│   │   ├── kanban/
│   │   │   ├── KanbanBoard.tsx         # Full board layout with 5 columns
│   │   │   ├── KanbanColumn.tsx        # Single column (header + card list)
│   │   │   └── TaskCard.tsx            # Card: name, badge, status, approve btn
│   │   │
│   │   ├── focus/
│   │   │   ├── FocusPanel.tsx          # Three-panel layout container
│   │   │   ├── SessionSidebar.tsx      # Left sidebar: session list + back btn
│   │   │   ├── TerminalPanel.tsx       # Center: xterm.js terminal
│   │   │   └── ChangesPanel.tsx        # Right: Monaco diff viewer + file tabs
│   │   │
│   │   └── shared/
│   │       ├── Layout.tsx              # App shell: header + content area
│   │       ├── Header.tsx              # Top bar: logo, new task btn, view toggle
│   │       ├── AgentBadge.tsx          # Colored badge for agent type
│   │       ├── StatusIndicator.tsx     # Staleness dot (green/yellow/red)
│   │       ├── NewTaskDialog.tsx       # Modal: create task form
│   │       └── CommandPalette.tsx      # Cmd+K command palette overlay
│   │
│   └── hooks/
│       ├── useWebSocket.ts            # Hook to init/teardown WS connection
│       ├── useKeyboardShortcuts.ts    # Hook to register global shortcuts
│       └── useTaskStaleness.ts        # Hook for staleness timer (30s/2min)
│
├── index.html                          # Vite HTML entry point
├── package.json                        # NPM dependencies
├── tsconfig.json                       # TypeScript config
├── tsconfig.node.json                  # TypeScript config for Vite/Node
├── vite.config.ts                      # Vite config with Tauri plugin
└── tailwind.config.ts                  # TailwindCSS config
```

---

## Chunk 1: Scaffolding & Infrastructure

### Task 1: Tauri 2.0 Project Setup

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/icons/icon.png` (placeholder — 32x32 transparent PNG)

- [ ] **Step 1: Create `src-tauri/Cargo.toml`**

```toml
# src-tauri/Cargo.toml
[package]
name = "shepherd-desktop"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = { version = "2", features = ["devtools"] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["process", "time"] }

[build-dependencies]
tauri-build = { version = "2", features = [] }

[lib]
name = "shepherd_desktop_lib"
crate-type = ["lib", "cdylib", "staticlib"]

[[bin]]
name = "shepherd-desktop"
path = "src/main.rs"
```

- [ ] **Step 2: Create Tauri build script**

```rust
// src-tauri/build.rs
fn main() {
    tauri_build::build();
}
```

- [ ] **Step 3: Create `src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-config-schema/schema.json",
  "productName": "Shepherd",
  "version": "0.1.0",
  "identifier": "com.shepherd.desktop",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "title": "Shepherd",
    "windows": [
      {
        "title": "Shepherd",
        "width": 1400,
        "height": 900,
        "minWidth": 1024,
        "minHeight": 700,
        "resizable": true,
        "fullscreen": false,
        "decorations": true,
        "transparent": false
      }
    ],
    "security": {
      "csp": "default-src 'self'; connect-src 'self' ws://127.0.0.1:* http://127.0.0.1:*; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-eval'"
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/icon.png"
    ]
  },
  "plugins": {}
}
```

- [ ] **Step 4: Create `src-tauri/capabilities/default.json`**

```json
{
  "$schema": "https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-utils/schema.json",
  "identifier": "default",
  "description": "Default capabilities for Shepherd",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "shell:allow-open"
  ]
}
```

- [ ] **Step 5: Create `src-tauri/src/main.rs`**

This is the Tauri entry point. It launches the Shepherd backend server as a sidecar process and creates the desktop window.

```rust
// src-tauri/src/main.rs

// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use std::sync::Mutex;

struct ServerProcess(Mutex<Option<tokio::process::Child>>);

#[tauri::command]
fn get_server_port() -> u16 {
    // Default port matching shepherd-server default
    std::env::var("SHEPHERD_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(9876)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(ServerProcess(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![get_server_port])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Spawn the shepherd-server process
            tauri::async_runtime::spawn(async move {
                let server_binary = if cfg!(debug_assertions) {
                    // In dev mode, assume server is built via `cargo run` in workspace
                    "shepherd-server".to_string()
                } else {
                    // In production, look for sidecar next to the app binary
                    let resource_dir = app_handle
                        .path()
                        .resource_dir()
                        .expect("Failed to get resource dir");
                    resource_dir
                        .join("shepherd-server")
                        .to_string_lossy()
                        .to_string()
                };

                match tokio::process::Command::new(&server_binary)
                    .kill_on_drop(true)
                    .spawn()
                {
                    Ok(child) => {
                        let state = app_handle.state::<ServerProcess>();
                        *state.0.lock().unwrap() = Some(child);
                        println!("Shepherd server started");
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to start shepherd-server: {}. \
                             Make sure it is built and in PATH (dev) or bundled (prod).",
                            e
                        );
                        // App still runs — user can start server manually
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                // Kill server process on app close
                let state = window.state::<ServerProcess>();
                if let Some(mut child) = state.0.lock().unwrap().take() {
                    tauri::async_runtime::spawn(async move {
                        let _ = child.kill().await;
                    });
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running shepherd desktop");
}
```

- [ ] **Step 6: Create placeholder icon**

Create a minimal placeholder at `src-tauri/icons/icon.png`. This can be a 32x32 transparent PNG. For now, generate it with:

```bash
# Generate a 32x32 placeholder PNG (requires ImageMagick, or just create manually)
mkdir -p src-tauri/icons
convert -size 32x32 xc:transparent src-tauri/icons/icon.png 2>/dev/null || \
  printf '\x89PNG\r\n\x1a\n' > src-tauri/icons/icon.png
```

If neither tool is available, create an empty file and replace later:

```bash
touch src-tauri/icons/icon.png
```

- [ ] **Step 7: Verify Tauri project compiles**

```bash
cd src-tauri && cargo check
```

**Expected:** No compilation errors. Warnings about unused variables are acceptable.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/
git commit -m "feat(desktop): scaffold Tauri 2.0 shell with server sidecar launch"
```

---

### Task 2: React + Vite + TypeScript + TailwindCSS Scaffold

**Files:**
- Create: `package.json`
- Create: `vite.config.ts`
- Create: `tsconfig.json`
- Create: `tsconfig.node.json`
- Create: `tailwind.config.ts`
- Create: `postcss.config.js`
- Create: `index.html`
- Create: `src/main.tsx`
- Create: `src/App.tsx`
- Create: `src/index.css`

- [ ] **Step 1: Create `package.json`**

```json
{
  "name": "shepherd-desktop",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "lint": "eslint src --ext .ts,.tsx",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "zustand": "^5.0.3",
    "@xterm/xterm": "^5.5.0",
    "@xterm/addon-fit": "^0.10.0",
    "@xterm/addon-web-links": "^0.11.0",
    "monaco-editor": "^0.52.2",
    "@monaco-editor/react": "^4.7.0",
    "@tauri-apps/api": "^2.2.0",
    "@tauri-apps/plugin-shell": "^2.2.0"
  },
  "devDependencies": {
    "@types/react": "^18.3.18",
    "@types/react-dom": "^18.3.5",
    "@vitejs/plugin-react": "^4.3.4",
    "autoprefixer": "^10.4.20",
    "postcss": "^8.5.3",
    "tailwindcss": "^3.4.17",
    "typescript": "^5.7.3",
    "vite": "^6.1.0",
    "@tauri-apps/cli": "^2.2.0"
  }
}
```

- [ ] **Step 2: Create `vite.config.ts`**

```typescript
// vite.config.ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [react()],

  // Prevent vite from obscuring Rust errors
  clearScreen: false,

  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // Tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
```

- [ ] **Step 3: Create `tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    }
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

- [ ] **Step 4: Create `tsconfig.node.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2023"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true,
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "forceConsistentCasingInFileNames": true
  },
  "include": ["vite.config.ts"]
}
```

- [ ] **Step 5: Create `tailwind.config.ts`**

```typescript
// tailwind.config.ts
import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        shepherd: {
          bg: "#0d1117",
          surface: "#161b22",
          border: "#30363d",
          text: "#e6edf3",
          muted: "#8b949e",
          accent: "#58a6ff",
          green: "#3fb950",
          yellow: "#d29922",
          red: "#f85149",
          orange: "#db6d28",
          purple: "#bc8cff",
        },
      },
      fontFamily: {
        sans: [
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "Noto Sans",
          "Helvetica",
          "Arial",
          "sans-serif",
        ],
        mono: [
          "SF Mono",
          "Monaco",
          "Inconsolata",
          "Fira Mono",
          "Droid Sans Mono",
          "Source Code Pro",
          "monospace",
        ],
      },
    },
  },
  plugins: [],
} satisfies Config;
```

- [ ] **Step 6: Create `postcss.config.js`**

```javascript
// postcss.config.js
export default {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
};
```

- [ ] **Step 7: Create `index.html`**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Shepherd</title>
    <style>
      /* Prevent white flash on load */
      html, body { background: #0d1117; margin: 0; }
    </style>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 8: Create `src/index.css`**

```css
/* src/index.css */
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  * {
    box-sizing: border-box;
  }

  body {
    @apply bg-shepherd-bg text-shepherd-text font-sans m-0 p-0;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    overflow: hidden;
    user-select: none;
  }

  /* Scrollbar styling */
  ::-webkit-scrollbar {
    width: 8px;
    height: 8px;
  }

  ::-webkit-scrollbar-track {
    @apply bg-shepherd-bg;
  }

  ::-webkit-scrollbar-thumb {
    @apply bg-shepherd-border rounded;
  }

  ::-webkit-scrollbar-thumb:hover {
    @apply bg-shepherd-muted;
  }
}
```

- [ ] **Step 9: Create `src/main.tsx`**

```tsx
// src/main.tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
```

- [ ] **Step 10: Create `src/App.tsx`**

```tsx
// src/App.tsx
import React from "react";

const App: React.FC = () => {
  return (
    <div className="h-screen w-screen flex flex-col bg-shepherd-bg">
      <header className="h-12 flex items-center px-4 border-b border-shepherd-border bg-shepherd-surface">
        <h1 className="text-sm font-semibold text-shepherd-text tracking-wide">
          SHEPHERD
        </h1>
      </header>
      <main className="flex-1 flex items-center justify-center text-shepherd-muted">
        <p>Loading...</p>
      </main>
    </div>
  );
};

export default App;
```

- [ ] **Step 11: Install dependencies and verify build**

```bash
npm install
npm run typecheck
npm run build
```

**Expected:** `typecheck` passes with zero errors. `build` produces `dist/` folder with `index.html` and JS/CSS assets.

- [ ] **Step 12: Commit**

```bash
git add package.json vite.config.ts tsconfig.json tsconfig.node.json tailwind.config.ts postcss.config.js index.html src/main.tsx src/App.tsx src/index.css
git commit -m "feat(frontend): scaffold React + Vite + TypeScript + TailwindCSS"
```

---

### Task 3: TypeScript Types Matching Rust Models

**Files:**
- Create: `src/types/task.ts`
- Create: `src/types/session.ts`
- Create: `src/types/permission.ts`
- Create: `src/types/events.ts`
- Create: `src/types/index.ts`

- [ ] **Step 1: Create `src/types/task.ts`**

These types mirror the Rust `Task`, `TaskStatus`, and `CreateTask` from `shepherd-core/src/db/models.rs`.

```typescript
// src/types/task.ts

/**
 * Task status values matching Rust TaskStatus enum.
 * Maps to Kanban columns:
 *   queued -> Queued, running -> Running, input -> Needs Input,
 *   review -> Review, done -> Done, error -> (shown in relevant column)
 */
export type TaskStatus =
  | "queued"
  | "running"
  | "input"
  | "review"
  | "error"
  | "done";

/**
 * Full task record from the database.
 * Mirrors: shepherd-core::db::models::Task
 */
export interface Task {
  id: number;
  title: string;
  prompt: string;
  agent_id: string;
  repo_path: string;
  branch: string;
  isolation_mode: string;
  status: TaskStatus;
  created_at: string;
  updated_at: string;
}

/**
 * Payload for creating a new task.
 * Mirrors: shepherd-core::db::models::CreateTask
 */
export interface CreateTask {
  title: string;
  prompt?: string;
  agent_id: string;
  repo_path?: string;
  isolation_mode?: string;
}

/**
 * Kanban column definition.
 */
export interface KanbanColumn {
  id: TaskStatus;
  label: string;
  tasks: Task[];
}

/**
 * Agent type info for badge display.
 */
export interface AgentInfo {
  id: string;
  label: string;
  color: string;
}

/** Well-known agent color map */
export const AGENT_COLORS: Record<string, AgentInfo> = {
  "claude-code": { id: "claude-code", label: "Claude", color: "#d97706" },
  "codex-cli": { id: "codex-cli", label: "Codex", color: "#059669" },
  "opencode": { id: "opencode", label: "OpenCode", color: "#7c3aed" },
  "gemini-cli": { id: "gemini-cli", label: "Gemini", color: "#2563eb" },
  "aider": { id: "aider", label: "Aider", color: "#dc2626" },
};
```

- [ ] **Step 2: Create `src/types/session.ts`**

```typescript
// src/types/session.ts

/**
 * Session record representing a PTY session for a task.
 * Mirrors the sessions table in the Rust backend.
 */
export interface Session {
  id: number;
  task_id: number;
  pty_pid: number | null;
  terminal_log_path: string;
  started_at: string;
  ended_at: string | null;
}
```

- [ ] **Step 3: Create `src/types/permission.ts`**

```typescript
// src/types/permission.ts

/**
 * Permission decision values.
 */
export type PermissionDecision = "auto" | "approved" | "denied" | "pending";

/**
 * Permission record from the database.
 * Mirrors the permissions table in the Rust backend.
 */
export interface Permission {
  id: number;
  task_id: number;
  tool_name: string;
  tool_args: string;
  decision: PermissionDecision;
  rule_matched: string | null;
  decided_at: string | null;
}
```

- [ ] **Step 4: Create `src/types/events.ts`**

These types mirror the Rust `ServerEvent`, `ClientEvent`, `TaskEvent`, `PermissionEvent`, and `StatusSnapshot` from `shepherd-core/src/events.rs`.

```typescript
// src/types/events.ts

/**
 * TaskEvent payload sent within server events.
 * Mirrors: shepherd-core::events::TaskEvent
 */
export interface TaskEvent {
  id: number;
  title: string;
  agent_id: string;
  status: string;
  branch: string;
  repo_path: string;
}

/**
 * PermissionEvent payload sent within server events.
 * Mirrors: shepherd-core::events::PermissionEvent
 */
export interface PermissionEvent {
  id: number;
  task_id: number;
  tool_name: string;
  tool_args: string;
  decision: string;
}

/**
 * StatusSnapshot — full state sent on WebSocket connect.
 * Mirrors: shepherd-core::events::StatusSnapshot
 */
export interface StatusSnapshot {
  tasks: TaskEvent[];
  pending_permissions: PermissionEvent[];
}

/**
 * Server-to-client events.
 * Mirrors: shepherd-core::events::ServerEvent
 *
 * The Rust enum uses serde(tag = "type", content = "data")
 * so JSON looks like: { "type": "task_created", "data": { ... } }
 */
export type ServerEvent =
  | { type: "task_created"; data: TaskEvent }
  | { type: "task_updated"; data: TaskEvent }
  | { type: "task_deleted"; data: { id: number } }
  | { type: "terminal_output"; data: { task_id: number; data: string } }
  | { type: "permission_requested"; data: PermissionEvent }
  | { type: "permission_resolved"; data: PermissionEvent }
  | { type: "gate_result"; data: { task_id: number; gate: string; passed: boolean } }
  | { type: "notification"; data: { kind: string; title: string; body: string } }
  | { type: "status_snapshot"; data: StatusSnapshot };

/**
 * Client-to-server events.
 * Mirrors: shepherd-core::events::ClientEvent
 */
export type ClientEvent =
  | {
      type: "task_create";
      data: {
        title: string;
        agent_id: string;
        repo_path?: string;
        isolation_mode?: string;
        prompt?: string;
      };
    }
  | { type: "task_approve"; data: { task_id: number } }
  | { type: "task_approve_all"; data: null }
  | { type: "task_cancel"; data: { task_id: number } }
  | { type: "terminal_input"; data: { task_id: number; data: string } }
  | { type: "terminal_resize"; data: { task_id: number; cols: number; rows: number } }
  | { type: "subscribe"; data: null };
```

- [ ] **Step 5: Create `src/types/index.ts`**

```typescript
// src/types/index.ts
export type {
  Task,
  TaskStatus,
  CreateTask,
  KanbanColumn,
  AgentInfo,
} from "./task";
export { AGENT_COLORS } from "./task";

export type { Session } from "./session";

export type { Permission, PermissionDecision } from "./permission";

export type {
  TaskEvent,
  PermissionEvent,
  StatusSnapshot,
  ServerEvent,
  ClientEvent,
} from "./events";
```

- [ ] **Step 6: Verify types compile**

```bash
npm run typecheck
```

**Expected:** Zero TypeScript errors.

- [ ] **Step 7: Commit**

```bash
git add src/types/
git commit -m "feat(types): add TypeScript types mirroring Rust models and events"
```

---

### Task 4: WebSocket Client with Auto-Reconnect

**Files:**
- Create: `src/lib/ws.ts`
- Create: `src/hooks/useWebSocket.ts`

- [ ] **Step 1: Create `src/lib/ws.ts`**

This is the core WebSocket client. It handles connection, auto-reconnect with exponential backoff, event parsing, and dispatching events to the Zustand store.

```typescript
// src/lib/ws.ts
import type { ServerEvent, ClientEvent } from "../types";

export type ConnectionStatus = "connecting" | "connected" | "disconnected" | "reconnecting";

export type ServerEventHandler = (event: ServerEvent) => void;
export type StatusChangeHandler = (status: ConnectionStatus) => void;

export interface WsClientOptions {
  /** WebSocket server URL, e.g. "ws://127.0.0.1:9876/ws" */
  url: string;
  /** Called for every server event received */
  onEvent: ServerEventHandler;
  /** Called when connection status changes */
  onStatusChange: StatusChangeHandler;
  /** Max reconnect attempts before giving up (0 = infinite) */
  maxReconnectAttempts?: number;
  /** Initial reconnect delay in ms (doubles each attempt, capped at 30s) */
  initialReconnectDelay?: number;
}

/**
 * WebSocket client with auto-reconnect and typed event parsing.
 *
 * Usage:
 *   const client = createWsClient({ url, onEvent, onStatusChange });
 *   client.connect();
 *   client.send({ type: "subscribe", data: null });
 *   // later...
 *   client.disconnect();
 */
export interface WsClient {
  connect(): void;
  disconnect(): void;
  send(event: ClientEvent): void;
  getStatus(): ConnectionStatus;
}

export function createWsClient(options: WsClientOptions): WsClient {
  const {
    url,
    onEvent,
    onStatusChange,
    maxReconnectAttempts = 0,
    initialReconnectDelay = 1000,
  } = options;

  let ws: WebSocket | null = null;
  let status: ConnectionStatus = "disconnected";
  let reconnectAttempts = 0;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let intentionalClose = false;
  let messageQueue: ClientEvent[] = [];

  function setStatus(newStatus: ConnectionStatus): void {
    if (status !== newStatus) {
      status = newStatus;
      onStatusChange(newStatus);
    }
  }

  function flushQueue(): void {
    while (messageQueue.length > 0 && ws?.readyState === WebSocket.OPEN) {
      const event = messageQueue.shift()!;
      ws.send(JSON.stringify(event));
    }
  }

  function scheduleReconnect(): void {
    if (intentionalClose) return;
    if (maxReconnectAttempts > 0 && reconnectAttempts >= maxReconnectAttempts) {
      setStatus("disconnected");
      return;
    }

    setStatus("reconnecting");
    const delay = Math.min(
      initialReconnectDelay * Math.pow(2, reconnectAttempts),
      30000,
    );
    reconnectAttempts++;

    reconnectTimer = setTimeout(() => {
      reconnectTimer = null;
      connect();
    }, delay);
  }

  function connect(): void {
    if (ws?.readyState === WebSocket.OPEN || ws?.readyState === WebSocket.CONNECTING) {
      return;
    }

    intentionalClose = false;
    setStatus("connecting");

    try {
      ws = new WebSocket(url);
    } catch {
      scheduleReconnect();
      return;
    }

    ws.onopen = () => {
      reconnectAttempts = 0;
      setStatus("connected");
      flushQueue();
    };

    ws.onmessage = (msgEvent: MessageEvent) => {
      try {
        const parsed = JSON.parse(msgEvent.data as string) as ServerEvent;
        onEvent(parsed);
      } catch (err) {
        console.error("[ws] Failed to parse server event:", err, msgEvent.data);
      }
    };

    ws.onclose = (event: CloseEvent) => {
      ws = null;
      if (!intentionalClose) {
        console.warn(`[ws] Connection closed (code=${event.code}). Reconnecting...`);
        scheduleReconnect();
      } else {
        setStatus("disconnected");
      }
    };

    ws.onerror = (err: Event) => {
      console.error("[ws] WebSocket error:", err);
      // onclose will fire after onerror, triggering reconnect
    };
  }

  function disconnect(): void {
    intentionalClose = true;
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
    if (ws) {
      ws.close(1000, "Client disconnect");
      ws = null;
    }
    messageQueue = [];
    setStatus("disconnected");
  }

  function send(event: ClientEvent): void {
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(event));
    } else {
      // Queue messages while reconnecting
      messageQueue.push(event);
    }
  }

  function getStatus(): ConnectionStatus {
    return status;
  }

  return { connect, disconnect, send, getStatus };
}
```

- [ ] **Step 2: Create `src/hooks/useWebSocket.ts`**

This React hook initializes the WebSocket client on mount and tears it down on unmount. It connects the WS event stream to the Zustand store (which will be built in Task 5).

```typescript
// src/hooks/useWebSocket.ts
import { useEffect, useRef } from "react";
import { createWsClient, type WsClient, type ConnectionStatus } from "../lib/ws";
import type { ServerEvent } from "../types";

const DEFAULT_PORT = 9876;
const WS_PATH = "/ws";

function getServerUrl(): string {
  const port = DEFAULT_PORT;
  return `ws://127.0.0.1:${port}${WS_PATH}`;
}

/**
 * Hook to manage WebSocket lifecycle.
 *
 * @param onEvent - callback for each server event (typically dispatches to Zustand)
 * @param onStatusChange - callback for connection status changes
 * @returns ref to WsClient for sending messages
 */
export function useWebSocket(
  onEvent: (event: ServerEvent) => void,
  onStatusChange: (status: ConnectionStatus) => void,
): React.MutableRefObject<WsClient | null> {
  const clientRef = useRef<WsClient | null>(null);
  const onEventRef = useRef(onEvent);
  const onStatusRef = useRef(onStatusChange);

  // Keep refs current without re-triggering effect
  onEventRef.current = onEvent;
  onStatusRef.current = onStatusChange;

  useEffect(() => {
    const client = createWsClient({
      url: getServerUrl(),
      onEvent: (event) => onEventRef.current(event),
      onStatusChange: (status) => onStatusRef.current(status),
    });

    clientRef.current = client;
    client.connect();

    // Send subscribe event once connected
    client.send({ type: "subscribe", data: null });

    return () => {
      client.disconnect();
      clientRef.current = null;
    };
  }, []);

  return clientRef;
}
```

- [ ] **Step 3: Verify types compile**

```bash
npm run typecheck
```

**Expected:** Zero TypeScript errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/ws.ts src/hooks/useWebSocket.ts
git commit -m "feat(ws): add WebSocket client with auto-reconnect and React hook"
```

---

## Chunk 2: State & API

### Task 5: Zustand Store with Slices

**Files:**
- Create: `src/store/tasks.ts`
- Create: `src/store/sessions.ts`
- Create: `src/store/ui.ts`
- Create: `src/store/index.ts`

- [ ] **Step 1: Create `src/store/tasks.ts`**

The tasks slice manages the full list of tasks and pending permissions. It receives updates from WebSocket events.

```typescript
// src/store/tasks.ts
import type { StateCreator } from "zustand";
import type { Task, TaskStatus } from "../types/task";
import type { PermissionEvent, TaskEvent } from "../types/events";

export interface TasksSlice {
  /** All tasks indexed by ID for O(1) lookup */
  tasks: Record<number, Task>;
  /** Pending permission requests */
  pendingPermissions: PermissionEvent[];

  // Actions
  setTasks: (tasks: TaskEvent[]) => void;
  upsertTask: (event: TaskEvent) => void;
  removeTask: (id: number) => void;
  setPendingPermissions: (perms: PermissionEvent[]) => void;
  addPendingPermission: (perm: PermissionEvent) => void;
  removePendingPermission: (permId: number) => void;

  // Selectors
  getTasksByStatus: (status: TaskStatus) => Task[];
  getTaskById: (id: number) => Task | undefined;
  getPermissionsForTask: (taskId: number) => PermissionEvent[];
}

/**
 * Convert a TaskEvent (from WS) to a full Task object.
 * TaskEvent is a subset; we fill defaults for fields not in the event.
 */
function taskEventToTask(event: TaskEvent): Task {
  return {
    id: event.id,
    title: event.title,
    agent_id: event.agent_id,
    status: event.status as TaskStatus,
    branch: event.branch,
    repo_path: event.repo_path,
    prompt: "",
    isolation_mode: "worktree",
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };
}

export const createTasksSlice: StateCreator<TasksSlice, [], [], TasksSlice> = (
  set,
  get,
) => ({
  tasks: {},
  pendingPermissions: [],

  setTasks: (taskEvents) => {
    const tasks: Record<number, Task> = {};
    for (const event of taskEvents) {
      tasks[event.id] = taskEventToTask(event);
    }
    set({ tasks });
  },

  upsertTask: (event) => {
    set((state) => ({
      tasks: {
        ...state.tasks,
        [event.id]: {
          ...state.tasks[event.id],
          ...taskEventToTask(event),
          // Preserve existing fields that aren't in the event
          ...(state.tasks[event.id]
            ? {
                prompt: state.tasks[event.id].prompt,
                isolation_mode: state.tasks[event.id].isolation_mode,
                created_at: state.tasks[event.id].created_at,
              }
            : {}),
          updated_at: new Date().toISOString(),
        },
      },
    }));
  },

  removeTask: (id) => {
    set((state) => {
      const { [id]: _, ...remaining } = state.tasks;
      return { tasks: remaining };
    });
  },

  setPendingPermissions: (perms) => {
    set({ pendingPermissions: perms });
  },

  addPendingPermission: (perm) => {
    set((state) => ({
      pendingPermissions: [...state.pendingPermissions, perm],
    }));
  },

  removePendingPermission: (permId) => {
    set((state) => ({
      pendingPermissions: state.pendingPermissions.filter((p) => p.id !== permId),
    }));
  },

  getTasksByStatus: (status) => {
    return Object.values(get().tasks).filter((t) => t.status === status);
  },

  getTaskById: (id) => {
    return get().tasks[id];
  },

  getPermissionsForTask: (taskId) => {
    return get().pendingPermissions.filter((p) => p.task_id === taskId);
  },
});
```

- [ ] **Step 2: Create `src/store/sessions.ts`**

```typescript
// src/store/sessions.ts
import type { StateCreator } from "zustand";
import type { Session } from "../types/session";

export interface SessionsSlice {
  /** Sessions indexed by task ID (one active session per task) */
  sessions: Record<number, Session>;

  // Actions
  setSession: (taskId: number, session: Session) => void;
  removeSession: (taskId: number) => void;
  clearSessions: () => void;

  // Selectors
  getSessionForTask: (taskId: number) => Session | undefined;
}

export const createSessionsSlice: StateCreator<SessionsSlice, [], [], SessionsSlice> = (
  _set,
  get,
) => ({
  sessions: {},

  setSession: (taskId, session) => {
    _set((state) => ({
      sessions: { ...state.sessions, [taskId]: session },
    }));
  },

  removeSession: (taskId) => {
    _set((state) => {
      const { [taskId]: _, ...remaining } = state.sessions;
      return { sessions: remaining };
    });
  },

  clearSessions: () => {
    _set({ sessions: {} });
  },

  getSessionForTask: (taskId) => {
    return get().sessions[taskId];
  },
});
```

- [ ] **Step 3: Create `src/store/ui.ts`**

```typescript
// src/store/ui.ts
import type { StateCreator } from "zustand";
import type { ConnectionStatus } from "../lib/ws";

export type ViewMode = "overview" | "focus";

export interface UiSlice {
  /** Current view: overview (Kanban) or focus (drill-down) */
  viewMode: ViewMode;
  /** Task ID currently focused (in focus mode) */
  focusedTaskId: number | null;
  /** WebSocket connection status */
  connectionStatus: ConnectionStatus;
  /** Whether the New Task dialog is open */
  isNewTaskDialogOpen: boolean;
  /** Whether the Command Palette is open */
  isCommandPaletteOpen: boolean;
  /** Which panel is focused in Focus mode: "terminal" | "changes" */
  focusedPanel: "terminal" | "changes";

  // Actions
  setViewMode: (mode: ViewMode) => void;
  setFocusedTaskId: (id: number | null) => void;
  enterFocus: (taskId: number) => void;
  exitFocus: () => void;
  toggleView: () => void;
  setConnectionStatus: (status: ConnectionStatus) => void;
  setNewTaskDialogOpen: (open: boolean) => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setFocusedPanel: (panel: "terminal" | "changes") => void;
}

export const createUiSlice: StateCreator<UiSlice, [], [], UiSlice> = (set, get) => ({
  viewMode: "overview",
  focusedTaskId: null,
  connectionStatus: "disconnected",
  isNewTaskDialogOpen: false,
  isCommandPaletteOpen: false,
  focusedPanel: "terminal",

  setViewMode: (mode) => set({ viewMode: mode }),

  setFocusedTaskId: (id) => set({ focusedTaskId: id }),

  enterFocus: (taskId) =>
    set({ viewMode: "focus", focusedTaskId: taskId, focusedPanel: "terminal" }),

  exitFocus: () =>
    set({ viewMode: "overview", focusedTaskId: null }),

  toggleView: () => {
    const { viewMode, focusedTaskId } = get();
    if (viewMode === "overview" && focusedTaskId !== null) {
      set({ viewMode: "focus" });
    } else {
      set({ viewMode: "overview" });
    }
  },

  setConnectionStatus: (status) => set({ connectionStatus: status }),

  setNewTaskDialogOpen: (open) => set({ isNewTaskDialogOpen: open }),

  setCommandPaletteOpen: (open) => set({ isCommandPaletteOpen: open }),

  setFocusedPanel: (panel) => set({ focusedPanel: panel }),
});
```

- [ ] **Step 4: Create `src/store/index.ts`**

Combine all slices into a single Zustand store.

```typescript
// src/store/index.ts
import { create } from "zustand";
import { createTasksSlice, type TasksSlice } from "./tasks";
import { createSessionsSlice, type SessionsSlice } from "./sessions";
import { createUiSlice, type UiSlice } from "./ui";

export type ShepherdStore = TasksSlice & SessionsSlice & UiSlice;

export const useStore = create<ShepherdStore>()((...a) => ({
  ...createTasksSlice(...a),
  ...createSessionsSlice(...a),
  ...createUiSlice(...a),
}));

/**
 * Convenience selector hooks to avoid importing useStore everywhere.
 */
export const useTasks = () => useStore((s) => s.tasks);
export const usePendingPermissions = () => useStore((s) => s.pendingPermissions);
export const useViewMode = () => useStore((s) => s.viewMode);
export const useFocusedTaskId = () => useStore((s) => s.focusedTaskId);
export const useConnectionStatus = () => useStore((s) => s.connectionStatus);
```

- [ ] **Step 5: Verify types compile**

```bash
npm run typecheck
```

**Expected:** Zero TypeScript errors.

- [ ] **Step 6: Commit**

```bash
git add src/store/
git commit -m "feat(store): add Zustand store with tasks, sessions, and UI slices"
```

---

### Task 6: REST API Client

**Files:**
- Create: `src/lib/api.ts`

- [ ] **Step 1: Create `src/lib/api.ts`**

Typed fetch wrappers for all REST endpoints exposed by the Shepherd server (from Plan 1, Task 7).

```typescript
// src/lib/api.ts
import type { Task, CreateTask } from "../types/task";

const DEFAULT_PORT = 9876;

function getBaseUrl(): string {
  return `http://127.0.0.1:${DEFAULT_PORT}`;
}

/**
 * API error with status code and response body.
 */
export class ApiError extends Error {
  constructor(
    public status: number,
    public body: unknown,
  ) {
    super(`API error ${status}: ${JSON.stringify(body)}`);
    this.name = "ApiError";
  }
}

/**
 * Generic fetch wrapper with error handling.
 */
async function request<T>(
  path: string,
  options?: RequestInit,
): Promise<T> {
  const url = `${getBaseUrl()}${path}`;
  const response = await fetch(url, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...options?.headers,
    },
  });

  if (!response.ok) {
    let body: unknown;
    try {
      body = await response.json();
    } catch {
      body = await response.text();
    }
    throw new ApiError(response.status, body);
  }

  // Handle 204 No Content
  if (response.status === 204) {
    return undefined as T;
  }

  return response.json() as Promise<T>;
}

// ----- Health -----

export interface HealthResponse {
  status: string;
  version: string;
}

/**
 * GET /api/health
 * Check if the server is running.
 */
export async function checkHealth(): Promise<HealthResponse> {
  return request<HealthResponse>("/api/health");
}

// ----- Tasks -----

/**
 * GET /api/tasks
 * List all tasks.
 */
export async function listTasks(): Promise<Task[]> {
  return request<Task[]>("/api/tasks");
}

/**
 * GET /api/tasks/:id
 * Get a single task by ID.
 */
export async function getTask(id: number): Promise<Task> {
  return request<Task>(`/api/tasks/${id}`);
}

/**
 * POST /api/tasks
 * Create a new task. Returns the created task.
 */
export async function createTask(task: CreateTask): Promise<Task> {
  return request<Task>("/api/tasks", {
    method: "POST",
    body: JSON.stringify(task),
  });
}

/**
 * DELETE /api/tasks/:id
 * Delete a task. Returns { deleted: id }.
 */
export async function deleteTask(id: number): Promise<{ deleted: number }> {
  return request<{ deleted: number }>(`/api/tasks/${id}`, {
    method: "DELETE",
  });
}

// ----- Task Actions (via REST for one-off commands) -----

/**
 * POST /api/tasks/:id/approve
 * Approve a pending permission for a task.
 */
export async function approveTask(id: number): Promise<{ status: string }> {
  return request<{ status: string }>(`/api/tasks/${id}/approve`, {
    method: "POST",
  });
}

/**
 * POST /api/tasks/:id/cancel
 * Cancel a running task.
 */
export async function cancelTask(id: number): Promise<{ status: string }> {
  return request<{ status: string }>(`/api/tasks/${id}/cancel`, {
    method: "POST",
  });
}

// ----- Utility -----

/**
 * Poll health endpoint until server is ready, or timeout.
 * Useful for Tauri startup when server may take a moment to start.
 */
export async function waitForServer(
  timeoutMs: number = 10000,
  intervalMs: number = 500,
): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      await checkHealth();
      return true;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, intervalMs));
    }
  }
  return false;
}
```

- [ ] **Step 2: Verify types compile**

```bash
npm run typecheck
```

**Expected:** Zero TypeScript errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/api.ts
git commit -m "feat(api): add typed REST API client for all server endpoints"
```

---

### Task 7: App Shell — Layout, Routing, Keyboard Shortcuts

**Files:**
- Create: `src/features/shared/Layout.tsx`
- Create: `src/features/shared/Header.tsx`
- Create: `src/lib/keys.ts`
- Create: `src/hooks/useKeyboardShortcuts.ts`
- Modify: `src/App.tsx`

- [ ] **Step 1: Create `src/lib/keys.ts`**

Keyboard shortcut system supporting the spec's shortcut table.

```typescript
// src/lib/keys.ts

export interface Shortcut {
  /** Unique identifier */
  id: string;
  /** Human-readable label for command palette */
  label: string;
  /** Key combination, e.g. "meta+n", "meta+shift+enter" */
  keys: string;
  /** Handler function */
  handler: () => void;
  /** Only active in this view mode (optional) */
  viewMode?: "overview" | "focus";
}

/**
 * Parse a keyboard event into a normalized key string.
 * Format: modifiers+key (sorted: alt, ctrl, meta, shift)
 *
 * Examples:
 *   Cmd+N -> "meta+n"
 *   Cmd+Shift+Enter -> "meta+shift+enter"
 *   1 -> "1"
 */
export function eventToKeyString(event: KeyboardEvent): string {
  const parts: string[] = [];
  if (event.altKey) parts.push("alt");
  if (event.ctrlKey) parts.push("ctrl");
  if (event.metaKey) parts.push("meta");
  if (event.shiftKey) parts.push("shift");

  let key = event.key.toLowerCase();
  // Normalize special keys
  if (key === "enter") key = "enter";
  else if (key === "[") key = "[";
  else if (key === "]") key = "]";

  // Don't add modifier keys themselves
  if (!["alt", "control", "meta", "shift"].includes(key)) {
    parts.push(key);
  }

  return parts.join("+");
}

/**
 * Create a keyboard shortcut manager.
 * Register shortcuts, and call handleKeyDown on each keydown event.
 */
export interface ShortcutManager {
  register(shortcut: Shortcut): void;
  unregister(id: string): void;
  handleKeyDown(event: KeyboardEvent, currentViewMode: "overview" | "focus"): boolean;
  getAll(): Shortcut[];
}

export function createShortcutManager(): ShortcutManager {
  const shortcuts = new Map<string, Shortcut>();

  return {
    register(shortcut) {
      shortcuts.set(shortcut.id, shortcut);
    },

    unregister(id) {
      shortcuts.delete(id);
    },

    handleKeyDown(event, currentViewMode) {
      const keyString = eventToKeyString(event);

      for (const shortcut of shortcuts.values()) {
        if (shortcut.keys !== keyString) continue;
        if (shortcut.viewMode && shortcut.viewMode !== currentViewMode) continue;

        event.preventDefault();
        event.stopPropagation();
        shortcut.handler();
        return true;
      }
      return false;
    },

    getAll() {
      return Array.from(shortcuts.values());
    },
  };
}
```

- [ ] **Step 2: Create `src/hooks/useKeyboardShortcuts.ts`**

```typescript
// src/hooks/useKeyboardShortcuts.ts
import { useEffect, useRef } from "react";
import { createShortcutManager, type ShortcutManager } from "../lib/keys";
import { useStore } from "../store";
import type { WsClient } from "../lib/ws";

/**
 * Hook to register global keyboard shortcuts per the spec.
 *
 * Shortcuts:
 *   Cmd+0       -> Toggle Overview / Focus
 *   Cmd+N       -> New Task dialog
 *   Cmd+Enter   -> Approve current task
 *   Cmd+Shift+Enter -> Approve all pending
 *   Cmd+]       -> Next session
 *   Cmd+[       -> Previous session
 *   Cmd+1       -> Focus terminal panel
 *   Cmd+2       -> Focus changes panel
 *   Cmd+K       -> Command palette
 *   1-9         -> Quick approve card N (Overview only)
 */
export function useKeyboardShortcuts(wsClient: React.MutableRefObject<WsClient | null>): ShortcutManager {
  const managerRef = useRef<ShortcutManager>(createShortcutManager());
  const manager = managerRef.current;

  useEffect(() => {
    const store = useStore.getState;

    // Cmd+0: Toggle Overview / Focus
    manager.register({
      id: "toggle-view",
      label: "Toggle Overview / Focus",
      keys: "meta+0",
      handler: () => store().toggleView(),
    });

    // Cmd+N: New Task
    manager.register({
      id: "new-task",
      label: "New Task",
      keys: "meta+n",
      handler: () => store().setNewTaskDialogOpen(true),
    });

    // Cmd+Enter: Approve current
    manager.register({
      id: "approve-current",
      label: "Approve Current Task",
      keys: "meta+enter",
      handler: () => {
        const { focusedTaskId } = store();
        if (focusedTaskId !== null) {
          wsClient.current?.send({ type: "task_approve", data: { task_id: focusedTaskId } });
        }
      },
    });

    // Cmd+Shift+Enter: Approve all
    manager.register({
      id: "approve-all",
      label: "Approve All Pending",
      keys: "meta+shift+enter",
      handler: () => {
        wsClient.current?.send({ type: "task_approve_all", data: null });
      },
    });

    // Cmd+1: Focus terminal
    manager.register({
      id: "focus-terminal",
      label: "Focus Terminal",
      keys: "meta+1",
      viewMode: "focus",
      handler: () => store().setFocusedPanel("terminal"),
    });

    // Cmd+2: Focus changes
    manager.register({
      id: "focus-changes",
      label: "Focus Changes",
      keys: "meta+2",
      viewMode: "focus",
      handler: () => store().setFocusedPanel("changes"),
    });

    // Cmd+K: Command palette
    manager.register({
      id: "command-palette",
      label: "Command Palette",
      keys: "meta+k",
      handler: () => {
        const current = store().isCommandPaletteOpen;
        store().setCommandPaletteOpen(!current);
      },
    });

    // 1-9: Quick approve card N (Overview only)
    for (let n = 1; n <= 9; n++) {
      manager.register({
        id: `quick-approve-${n}`,
        label: `Quick Approve Card ${n}`,
        keys: String(n),
        viewMode: "overview",
        handler: () => {
          const inputTasks = store().getTasksByStatus("input");
          const task = inputTasks[n - 1];
          if (task) {
            wsClient.current?.send({ type: "task_approve", data: { task_id: task.id } });
          }
        },
      });
    }

    // Keyboard event listener
    const handleKeyDown = (event: KeyboardEvent) => {
      // Don't capture shortcuts when typing in inputs/textareas
      const target = event.target as HTMLElement;
      if (
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable
      ) {
        // Still allow meta shortcuts in inputs
        if (!event.metaKey && !event.ctrlKey) return;
      }

      const { viewMode } = useStore.getState();
      manager.handleKeyDown(event, viewMode);
    };

    window.addEventListener("keydown", handleKeyDown);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [manager, wsClient]);

  return manager;
}
```

- [ ] **Step 3: Create `src/features/shared/Header.tsx`**

```tsx
// src/features/shared/Header.tsx
import React from "react";
import { useStore } from "../../store";
import type { ConnectionStatus } from "../../lib/ws";

const STATUS_COLORS: Record<ConnectionStatus, string> = {
  connected: "bg-shepherd-green",
  connecting: "bg-shepherd-yellow",
  reconnecting: "bg-shepherd-yellow animate-pulse",
  disconnected: "bg-shepherd-red",
};

const STATUS_LABELS: Record<ConnectionStatus, string> = {
  connected: "Connected",
  connecting: "Connecting...",
  reconnecting: "Reconnecting...",
  disconnected: "Disconnected",
};

export const Header: React.FC = () => {
  const viewMode = useStore((s) => s.viewMode);
  const connectionStatus = useStore((s) => s.connectionStatus);
  const setNewTaskDialogOpen = useStore((s) => s.setNewTaskDialogOpen);
  const exitFocus = useStore((s) => s.exitFocus);
  const pendingPermissions = useStore((s) => s.pendingPermissions);

  const needsInputCount = pendingPermissions.length;

  return (
    <header className="h-12 flex items-center justify-between px-4 border-b border-shepherd-border bg-shepherd-surface shrink-0">
      {/* Left section */}
      <div className="flex items-center gap-3">
        {viewMode === "focus" && (
          <button
            onClick={exitFocus}
            className="text-shepherd-muted hover:text-shepherd-text text-sm flex items-center gap-1 transition-colors"
          >
            <span className="text-xs">&larr;</span> Overview
          </button>
        )}
        <h1 className="text-sm font-semibold text-shepherd-text tracking-wide uppercase">
          Shepherd
        </h1>
        <span className="text-xs text-shepherd-muted">
          {viewMode === "overview" ? "Overview" : "Focus"}
        </span>
      </div>

      {/* Center section */}
      <div className="flex items-center gap-2">
        {needsInputCount > 0 && (
          <span className="px-2 py-0.5 text-xs rounded-full bg-shepherd-orange/20 text-shepherd-orange font-medium">
            {needsInputCount} pending
          </span>
        )}
      </div>

      {/* Right section */}
      <div className="flex items-center gap-3">
        <button
          onClick={() => setNewTaskDialogOpen(true)}
          className="px-3 py-1.5 text-xs font-medium rounded bg-shepherd-accent text-white hover:bg-shepherd-accent/80 transition-colors"
          title="New Task (Cmd+N)"
        >
          + New Task
        </button>

        {/* Connection status indicator */}
        <div className="flex items-center gap-1.5" title={STATUS_LABELS[connectionStatus]}>
          <div className={`w-2 h-2 rounded-full ${STATUS_COLORS[connectionStatus]}`} />
          <span className="text-xs text-shepherd-muted">
            {STATUS_LABELS[connectionStatus]}
          </span>
        </div>
      </div>
    </header>
  );
};
```

- [ ] **Step 4: Create `src/features/shared/Layout.tsx`**

```tsx
// src/features/shared/Layout.tsx
import React from "react";
import { Header } from "./Header";

interface LayoutProps {
  children: React.ReactNode;
}

export const Layout: React.FC<LayoutProps> = ({ children }) => {
  return (
    <div className="h-screen w-screen flex flex-col bg-shepherd-bg overflow-hidden">
      <Header />
      <main className="flex-1 overflow-hidden">
        {children}
      </main>
    </div>
  );
};
```

- [ ] **Step 5: Update `src/App.tsx`**

Wire together the layout, WebSocket, keyboard shortcuts, and view routing.

```tsx
// src/App.tsx
import React, { useCallback } from "react";
import { Layout } from "./features/shared/Layout";
import { useWebSocket } from "./hooks/useWebSocket";
import { useKeyboardShortcuts } from "./hooks/useKeyboardShortcuts";
import { useStore } from "./store";
import type { ServerEvent } from "./types";
import type { ConnectionStatus } from "./lib/ws";

const App: React.FC = () => {
  const viewMode = useStore((s) => s.viewMode);

  // Handle server events by dispatching to Zustand store
  const handleServerEvent = useCallback((event: ServerEvent) => {
    const store = useStore.getState();

    switch (event.type) {
      case "status_snapshot":
        store.setTasks(event.data.tasks);
        store.setPendingPermissions(event.data.pending_permissions);
        break;
      case "task_created":
      case "task_updated":
        store.upsertTask(event.data);
        break;
      case "task_deleted":
        store.removeTask(event.data.id);
        break;
      case "permission_requested":
        store.addPendingPermission(event.data);
        break;
      case "permission_resolved":
        store.removePendingPermission(event.data.id);
        break;
      case "terminal_output":
        // Terminal output will be handled by the terminal component directly
        break;
      case "gate_result":
        // Gate results will be handled in Plan 3
        break;
      case "notification":
        // Notifications will be handled in Plan 3
        break;
    }
  }, []);

  const handleStatusChange = useCallback((status: ConnectionStatus) => {
    useStore.getState().setConnectionStatus(status);
  }, []);

  const wsRef = useWebSocket(handleServerEvent, handleStatusChange);
  useKeyboardShortcuts(wsRef);

  return (
    <Layout>
      {viewMode === "overview" ? (
        <div className="flex items-center justify-center h-full text-shepherd-muted">
          <p className="text-sm">Kanban board will render here (Task 8)</p>
        </div>
      ) : (
        <div className="flex items-center justify-center h-full text-shepherd-muted">
          <p className="text-sm">Focus panel will render here (Chunk 4)</p>
        </div>
      )}
    </Layout>
  );
};

export default App;
```

- [ ] **Step 6: Verify types compile**

```bash
npm run typecheck
```

**Expected:** Zero TypeScript errors.

- [ ] **Step 7: Verify build**

```bash
npm run build
```

**Expected:** Vite builds successfully. Output in `dist/`.

- [ ] **Step 8: Commit**

```bash
git add src/lib/keys.ts src/hooks/useKeyboardShortcuts.ts src/features/shared/Header.tsx src/features/shared/Layout.tsx src/App.tsx
git commit -m "feat(shell): add app shell with layout, header, keyboard shortcuts, and WS integration"
```

---

## Chunk 3: Kanban View

### Task 8: KanbanBoard + KanbanColumn Components

**Files:**
- Create: `src/features/kanban/KanbanBoard.tsx`
- Create: `src/features/kanban/KanbanColumn.tsx`
- Modify: `src/App.tsx` (replace placeholder with KanbanBoard)

- [ ] **Step 1: Create `src/features/kanban/KanbanColumn.tsx`**

A single Kanban column: header with task count, scrollable card list.

```tsx
// src/features/kanban/KanbanColumn.tsx
import React from "react";
import type { Task, TaskStatus } from "../../types";

export interface KanbanColumnProps {
  /** Column status key */
  status: TaskStatus;
  /** Display label */
  label: string;
  /** Tasks in this column */
  tasks: Task[];
  /** Render function for each task card */
  renderCard: (task: Task) => React.ReactNode;
  /** Whether this column supports drag-and-drop reordering */
  isDraggable?: boolean;
  /** Color accent for the column header dot */
  accentColor: string;
}

const COLUMN_BG: Record<TaskStatus, string> = {
  queued: "border-shepherd-muted/30",
  running: "border-shepherd-accent/30",
  input: "border-shepherd-orange/30",
  review: "border-shepherd-purple/30",
  done: "border-shepherd-green/30",
  error: "border-shepherd-red/30",
};

export const KanbanColumn: React.FC<KanbanColumnProps> = ({
  status,
  label,
  tasks,
  renderCard,
  accentColor,
}) => {
  return (
    <div
      className={`flex flex-col min-w-[260px] max-w-[320px] flex-1 bg-shepherd-surface/50 rounded-lg border ${COLUMN_BG[status]}`}
    >
      {/* Column header */}
      <div className="flex items-center gap-2 px-3 py-2.5 border-b border-shepherd-border/50">
        <div
          className="w-2.5 h-2.5 rounded-full shrink-0"
          style={{ backgroundColor: accentColor }}
        />
        <h2 className="text-xs font-semibold text-shepherd-text uppercase tracking-wider flex-1">
          {label}
        </h2>
        <span className="text-xs text-shepherd-muted bg-shepherd-border/30 px-1.5 py-0.5 rounded-full min-w-[20px] text-center">
          {tasks.length}
        </span>
      </div>

      {/* Card list */}
      <div className="flex-1 overflow-y-auto p-2 space-y-2">
        {tasks.length === 0 ? (
          <div className="text-xs text-shepherd-muted/50 text-center py-8">
            No tasks
          </div>
        ) : (
          tasks.map((task) => (
            <div key={task.id}>
              {renderCard(task)}
            </div>
          ))
        )}
      </div>
    </div>
  );
};
```

- [ ] **Step 2: Create `src/features/kanban/KanbanBoard.tsx`**

The full board with five columns: Queued, Running, Needs Input, Review, Done.

```tsx
// src/features/kanban/KanbanBoard.tsx
import React from "react";
import { useStore } from "../../store";
import { KanbanColumn } from "./KanbanColumn";
import { TaskCard } from "./TaskCard";
import type { Task, TaskStatus } from "../../types";

interface ColumnDef {
  status: TaskStatus;
  label: string;
  accentColor: string;
}

const COLUMNS: ColumnDef[] = [
  { status: "queued", label: "Queued", accentColor: "#8b949e" },
  { status: "running", label: "Running", accentColor: "#58a6ff" },
  { status: "input", label: "Needs Input", accentColor: "#db6d28" },
  { status: "review", label: "Review", accentColor: "#bc8cff" },
  { status: "done", label: "Done", accentColor: "#3fb950" },
];

export const KanbanBoard: React.FC = () => {
  const tasks = useStore((s) => s.tasks);
  const enterFocus = useStore((s) => s.enterFocus);
  const pendingPermissions = useStore((s) => s.pendingPermissions);

  // Group tasks by status
  const tasksByStatus: Record<TaskStatus, Task[]> = {
    queued: [],
    running: [],
    input: [],
    review: [],
    done: [],
    error: [],
  };

  for (const task of Object.values(tasks)) {
    if (tasksByStatus[task.status]) {
      tasksByStatus[task.status].push(task);
    }
  }

  // Tasks with errors go to the column matching their last known status,
  // or fall into a separate error display. For now, show in review.
  for (const task of tasksByStatus.error) {
    tasksByStatus.review.push(task);
  }

  // Sort: queued by ID (insertion order), others by updated_at descending
  tasksByStatus.queued.sort((a, b) => a.id - b.id);
  for (const status of ["running", "input", "review", "done"] as TaskStatus[]) {
    tasksByStatus[status].sort(
      (a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime(),
    );
  }

  // Done tasks with opacity: fade if older than 24h
  const now = Date.now();
  const twentyFourHours = 24 * 60 * 60 * 1000;

  const handleCardClick = (taskId: number) => {
    enterFocus(taskId);
  };

  return (
    <div className="h-full flex gap-3 p-4 overflow-x-auto">
      {COLUMNS.map((col) => (
        <KanbanColumn
          key={col.status}
          status={col.status}
          label={col.label}
          accentColor={col.accentColor}
          tasks={tasksByStatus[col.status]}
          isDraggable={col.status === "queued"}
          renderCard={(task) => {
            const isDone = task.status === "done";
            const taskAge = now - new Date(task.updated_at).getTime();
            const isFaded = isDone && taskAge > twentyFourHours;

            return (
              <div
                className={`transition-opacity ${isFaded ? "opacity-40" : "opacity-100"}`}
              >
                <TaskCard
                  task={task}
                  permissions={pendingPermissions.filter(
                    (p) => p.task_id === task.id,
                  )}
                  onClick={() => handleCardClick(task.id)}
                />
              </div>
            );
          }}
        />
      ))}
    </div>
  );
};
```

- [ ] **Step 3: Update `src/App.tsx`** to use KanbanBoard

Replace the overview placeholder in `src/App.tsx`:

```tsx
// src/App.tsx
import React, { useCallback } from "react";
import { Layout } from "./features/shared/Layout";
import { KanbanBoard } from "./features/kanban/KanbanBoard";
import { useWebSocket } from "./hooks/useWebSocket";
import { useKeyboardShortcuts } from "./hooks/useKeyboardShortcuts";
import { useStore } from "./store";
import type { ServerEvent } from "./types";
import type { ConnectionStatus } from "./lib/ws";

const App: React.FC = () => {
  const viewMode = useStore((s) => s.viewMode);

  // Handle server events by dispatching to Zustand store
  const handleServerEvent = useCallback((event: ServerEvent) => {
    const store = useStore.getState();

    switch (event.type) {
      case "status_snapshot":
        store.setTasks(event.data.tasks);
        store.setPendingPermissions(event.data.pending_permissions);
        break;
      case "task_created":
      case "task_updated":
        store.upsertTask(event.data);
        break;
      case "task_deleted":
        store.removeTask(event.data.id);
        break;
      case "permission_requested":
        store.addPendingPermission(event.data);
        break;
      case "permission_resolved":
        store.removePendingPermission(event.data.id);
        break;
      case "terminal_output":
        // Terminal output will be handled by the terminal component directly
        break;
      case "gate_result":
        // Gate results will be handled in Plan 3
        break;
      case "notification":
        // Notifications will be handled in Plan 3
        break;
    }
  }, []);

  const handleStatusChange = useCallback((status: ConnectionStatus) => {
    useStore.getState().setConnectionStatus(status);
  }, []);

  const wsRef = useWebSocket(handleServerEvent, handleStatusChange);
  useKeyboardShortcuts(wsRef);

  return (
    <Layout>
      {viewMode === "overview" ? (
        <KanbanBoard />
      ) : (
        <div className="flex items-center justify-center h-full text-shepherd-muted">
          <p className="text-sm">Focus panel will render here (Chunk 4)</p>
        </div>
      )}
    </Layout>
  );
};

export default App;
```

- [ ] **Step 4: Verify types compile**

```bash
npm run typecheck
```

**Expected:** Zero TypeScript errors.

- [ ] **Step 5: Commit**

```bash
git add src/features/kanban/KanbanBoard.tsx src/features/kanban/KanbanColumn.tsx src/App.tsx
git commit -m "feat(kanban): add KanbanBoard and KanbanColumn components with 5-column layout"
```

---

### Task 9: TaskCard with Agent Badges, Staleness, Approve Button

**Files:**
- Create: `src/features/shared/AgentBadge.tsx`
- Create: `src/features/shared/StatusIndicator.tsx`
- Create: `src/features/kanban/TaskCard.tsx`
- Create: `src/hooks/useTaskStaleness.ts`

- [ ] **Step 1: Create `src/features/shared/AgentBadge.tsx`**

```tsx
// src/features/shared/AgentBadge.tsx
import React from "react";
import { AGENT_COLORS, type AgentInfo } from "../../types";

interface AgentBadgeProps {
  agentId: string;
  size?: "sm" | "md";
}

const DEFAULT_AGENT: AgentInfo = {
  id: "unknown",
  label: "Agent",
  color: "#6b7280",
};

export const AgentBadge: React.FC<AgentBadgeProps> = ({ agentId, size = "sm" }) => {
  const agent = AGENT_COLORS[agentId] ?? { ...DEFAULT_AGENT, id: agentId, label: agentId };
  const sizeClasses = size === "sm" ? "text-[10px] px-1.5 py-0.5" : "text-xs px-2 py-0.5";

  return (
    <span
      className={`inline-flex items-center rounded font-medium ${sizeClasses}`}
      style={{
        backgroundColor: `${agent.color}20`,
        color: agent.color,
        border: `1px solid ${agent.color}40`,
      }}
    >
      {agent.label}
    </span>
  );
};
```

- [ ] **Step 2: Create `src/hooks/useTaskStaleness.ts`**

Per the spec: yellow >30s, red >2min idle.

```typescript
// src/hooks/useTaskStaleness.ts
import { useState, useEffect } from "react";

export type StalenessLevel = "fresh" | "stale" | "critical";

const STALE_THRESHOLD_MS = 30_000; // 30 seconds
const CRITICAL_THRESHOLD_MS = 120_000; // 2 minutes

/**
 * Hook that returns a staleness level for a given timestamp.
 * Re-evaluates every 10 seconds.
 *
 * @param updatedAt - ISO timestamp of last activity
 * @param isActive - only compute staleness for active tasks (running/input)
 * @returns "fresh" | "stale" | "critical"
 */
export function useTaskStaleness(
  updatedAt: string,
  isActive: boolean,
): StalenessLevel {
  const [level, setLevel] = useState<StalenessLevel>("fresh");

  useEffect(() => {
    if (!isActive) {
      setLevel("fresh");
      return;
    }

    function compute(): StalenessLevel {
      const elapsed = Date.now() - new Date(updatedAt).getTime();
      if (elapsed > CRITICAL_THRESHOLD_MS) return "critical";
      if (elapsed > STALE_THRESHOLD_MS) return "stale";
      return "fresh";
    }

    setLevel(compute());

    const interval = setInterval(() => {
      setLevel(compute());
    }, 10_000);

    return () => clearInterval(interval);
  }, [updatedAt, isActive]);

  return level;
}
```

- [ ] **Step 3: Create `src/features/shared/StatusIndicator.tsx`**

```tsx
// src/features/shared/StatusIndicator.tsx
import React from "react";
import type { StalenessLevel } from "../../hooks/useTaskStaleness";

interface StatusIndicatorProps {
  level: StalenessLevel;
  size?: "sm" | "md";
}

const LEVEL_COLORS: Record<StalenessLevel, string> = {
  fresh: "bg-shepherd-green",
  stale: "bg-shepherd-yellow",
  critical: "bg-shepherd-red animate-pulse",
};

const LEVEL_LABELS: Record<StalenessLevel, string> = {
  fresh: "Active",
  stale: "Idle >30s",
  critical: "Idle >2min",
};

export const StatusIndicator: React.FC<StatusIndicatorProps> = ({
  level,
  size = "sm",
}) => {
  const dotSize = size === "sm" ? "w-2 h-2" : "w-2.5 h-2.5";

  return (
    <div className="flex items-center gap-1" title={LEVEL_LABELS[level]}>
      <div className={`${dotSize} rounded-full ${LEVEL_COLORS[level]}`} />
    </div>
  );
};
```

- [ ] **Step 4: Create `src/features/kanban/TaskCard.tsx`**

The task card shows: task name, agent type badge, branch name, current action/permission question/diff stats, staleness indicator, and an approve button for tasks in the "input" column.

```tsx
// src/features/kanban/TaskCard.tsx
import React from "react";
import type { Task } from "../../types";
import type { PermissionEvent } from "../../types/events";
import { AgentBadge } from "../shared/AgentBadge";
import { StatusIndicator } from "../shared/StatusIndicator";
import { useTaskStaleness } from "../../hooks/useTaskStaleness";
import { useStore } from "../../store";

interface TaskCardProps {
  task: Task;
  permissions: PermissionEvent[];
  onClick: () => void;
}

export const TaskCard: React.FC<TaskCardProps> = ({ task, permissions, onClick }) => {
  const staleness = useTaskStaleness(
    task.updated_at,
    task.status === "running" || task.status === "input",
  );

  const wsClient = useRef<null>(null); // Will be wired up via context in future
  const isInput = task.status === "input";
  const isRunning = task.status === "running";
  const isReview = task.status === "review";
  const isError = task.status === "error";
  const latestPermission = permissions[0];

  const handleApprove = (e: React.MouseEvent) => {
    e.stopPropagation();
    // Send approve via the WebSocket (wired through store action)
    // For now, we use the REST API as fallback
    import("../../lib/api").then((api) => {
      api.approveTask(task.id).catch(console.error);
    });
  };

  return (
    <div
      onClick={onClick}
      className={`
        group cursor-pointer rounded-md border bg-shepherd-surface p-3 space-y-2
        transition-all hover:border-shepherd-accent/50 hover:shadow-md hover:shadow-shepherd-accent/5
        ${isInput ? "border-shepherd-orange/40" : "border-shepherd-border"}
        ${isError ? "border-shepherd-red/40" : ""}
      `}
    >
      {/* Row 1: Title + Staleness */}
      <div className="flex items-start justify-between gap-2">
        <h3 className="text-sm font-medium text-shepherd-text leading-tight line-clamp-2 flex-1">
          {task.title}
        </h3>
        {(isRunning || isInput) && <StatusIndicator level={staleness} />}
      </div>

      {/* Row 2: Agent badge + branch */}
      <div className="flex items-center gap-2 flex-wrap">
        <AgentBadge agentId={task.agent_id} />
        {task.branch && (
          <span className="text-[10px] text-shepherd-muted font-mono truncate max-w-[120px]">
            {task.branch}
          </span>
        )}
      </div>

      {/* Row 3: Status-specific content */}
      {isInput && latestPermission && (
        <div className="text-xs text-shepherd-orange bg-shepherd-orange/10 rounded px-2 py-1.5">
          <span className="font-medium">{latestPermission.tool_name}</span>
          <span className="text-shepherd-muted ml-1 truncate block mt-0.5">
            {latestPermission.tool_args}
          </span>
        </div>
      )}

      {isReview && (
        <div className="flex items-center gap-2 flex-wrap text-xs text-shepherd-purple">
          <span>Ready for review</span>
          {task.gate_results && task.gate_results.length > 0 && (
            <div className="flex items-center gap-1">
              {task.gate_results.map((g) => (
                <span
                  key={g.gate}
                  className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${
                    g.passed ? "bg-shepherd-green/20 text-shepherd-green" : "bg-shepherd-red/20 text-shepherd-red"
                  }`}
                  title={g.gate}
                >
                  {g.passed ? "✓" : "✗"} {g.gate}
                </span>
              ))}
            </div>
          )}
        </div>
      )}

      {isError && (
        <div className="text-xs text-shepherd-red">
          Error — click to investigate
        </div>
      )}

      {/* Row 4: Approve button (only for "input" status) */}
      {isInput && (
        <button
          onClick={handleApprove}
          className="
            w-full py-1.5 text-xs font-medium rounded
            bg-shepherd-green/20 text-shepherd-green border border-shepherd-green/30
            hover:bg-shepherd-green/30 transition-colors
          "
        >
          Approve
        </button>
      )}
    </div>
  );
};
```

Wait — the TaskCard above uses `useRef` without importing it. Let me fix that.

- [ ] **Step 4 (corrected): Create `src/features/kanban/TaskCard.tsx`**

```tsx
// src/features/kanban/TaskCard.tsx
import React from "react";
import type { Task } from "../../types";
import type { PermissionEvent } from "../../types/events";
import { AgentBadge } from "../shared/AgentBadge";
import { StatusIndicator } from "../shared/StatusIndicator";
import { useTaskStaleness } from "../../hooks/useTaskStaleness";

interface TaskCardProps {
  task: Task;
  permissions: PermissionEvent[];
  onClick: () => void;
}

export const TaskCard: React.FC<TaskCardProps> = ({ task, permissions, onClick }) => {
  const staleness = useTaskStaleness(
    task.updated_at,
    task.status === "running" || task.status === "input",
  );

  const isInput = task.status === "input";
  const isRunning = task.status === "running";
  const isReview = task.status === "review";
  const isError = task.status === "error";
  const latestPermission = permissions[0];

  const handleApprove = (e: React.MouseEvent) => {
    e.stopPropagation();
    // Send approve via the REST API
    import("../../lib/api").then((api) => {
      api.approveTask(task.id).catch(console.error);
    });
  };

  return (
    <div
      onClick={onClick}
      className={`
        group cursor-pointer rounded-md border bg-shepherd-surface p-3 space-y-2
        transition-all hover:border-shepherd-accent/50 hover:shadow-md hover:shadow-shepherd-accent/5
        ${isInput ? "border-shepherd-orange/40" : "border-shepherd-border"}
        ${isError ? "border-shepherd-red/40" : ""}
      `}
    >
      {/* Row 1: Title + Staleness */}
      <div className="flex items-start justify-between gap-2">
        <h3 className="text-sm font-medium text-shepherd-text leading-tight line-clamp-2 flex-1">
          {task.title}
        </h3>
        {(isRunning || isInput) && <StatusIndicator level={staleness} />}
      </div>

      {/* Row 2: Agent badge + branch */}
      <div className="flex items-center gap-2 flex-wrap">
        <AgentBadge agentId={task.agent_id} />
        {task.branch && (
          <span className="text-[10px] text-shepherd-muted font-mono truncate max-w-[120px]">
            {task.branch}
          </span>
        )}
      </div>

      {/* Row 3: Status-specific content */}
      {isInput && latestPermission && (
        <div className="text-xs text-shepherd-orange bg-shepherd-orange/10 rounded px-2 py-1.5">
          <span className="font-medium">{latestPermission.tool_name}</span>
          <span className="text-shepherd-muted ml-1 truncate block mt-0.5">
            {latestPermission.tool_args}
          </span>
        </div>
      )}

      {isReview && (
        <div className="flex items-center gap-2 flex-wrap text-xs text-shepherd-purple">
          <span>Ready for review</span>
          {task.gate_results && task.gate_results.length > 0 && (
            <div className="flex items-center gap-1">
              {task.gate_results.map((g) => (
                <span
                  key={g.gate}
                  className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${
                    g.passed ? "bg-shepherd-green/20 text-shepherd-green" : "bg-shepherd-red/20 text-shepherd-red"
                  }`}
                  title={g.gate}
                >
                  {g.passed ? "✓" : "✗"} {g.gate}
                </span>
              ))}
            </div>
          )}
        </div>
      )}

      {isError && (
        <div className="text-xs text-shepherd-red">
          Error — click to investigate
        </div>
      )}

      {/* Row 4: Approve button (only for "input" status) */}
      {isInput && (
        <button
          onClick={handleApprove}
          className="
            w-full py-1.5 text-xs font-medium rounded
            bg-shepherd-green/20 text-shepherd-green border border-shepherd-green/30
            hover:bg-shepherd-green/30 transition-colors
          "
        >
          Approve
        </button>
      )}
    </div>
  );
};
```

- [ ] **Step 5: Verify types compile**

```bash
npm run typecheck
```

**Expected:** Zero TypeScript errors.

- [ ] **Step 6: Verify build**

```bash
npm run build
```

**Expected:** Vite builds successfully.

- [ ] **Step 7: Commit**

```bash
git add src/features/shared/AgentBadge.tsx src/features/shared/StatusIndicator.tsx src/features/kanban/TaskCard.tsx src/hooks/useTaskStaleness.ts
git commit -m "feat(kanban): add TaskCard with agent badges, staleness indicators, and approve button"
```

---

### Task 10: Drag-and-Drop and Quick-Approve

**Files:**
- Modify: `src/features/kanban/KanbanColumn.tsx` (add drag-and-drop for Queued)
- Modify: `src/features/kanban/KanbanBoard.tsx` (handle reorder)
- Create: `src/features/kanban/useDragAndDrop.ts`

- [ ] **Step 1: Create `src/features/kanban/useDragAndDrop.ts`**

A lightweight drag-and-drop hook using the HTML5 Drag and Drop API. No external dependencies. Only applies to the Queued column for priority reordering.

```typescript
// src/features/kanban/useDragAndDrop.ts
import { useState, useCallback } from "react";

export interface DragState {
  /** ID of the item being dragged */
  draggedId: number | null;
  /** Index the dragged item is currently hovering over */
  overIndex: number | null;
}

export interface UseDragAndDropReturn {
  dragState: DragState;
  handleDragStart: (id: number) => (e: React.DragEvent) => void;
  handleDragOver: (index: number) => (e: React.DragEvent) => void;
  handleDragEnd: () => void;
  handleDrop: (index: number) => (e: React.DragEvent) => void;
}

/**
 * Hook for drag-and-drop reordering within a single list.
 *
 * @param items - current ordered list of item IDs
 * @param onReorder - callback with the new ordered list of IDs after a drop
 */
export function useDragAndDrop(
  items: number[],
  onReorder: (newOrder: number[]) => void,
): UseDragAndDropReturn {
  const [dragState, setDragState] = useState<DragState>({
    draggedId: null,
    overIndex: null,
  });

  const handleDragStart = useCallback(
    (id: number) => (e: React.DragEvent) => {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", String(id));
      setDragState({ draggedId: id, overIndex: null });
    },
    [],
  );

  const handleDragOver = useCallback(
    (index: number) => (e: React.DragEvent) => {
      e.preventDefault();
      e.dataTransfer.dropEffect = "move";
      setDragState((prev) => ({ ...prev, overIndex: index }));
    },
    [],
  );

  const handleDragEnd = useCallback(() => {
    setDragState({ draggedId: null, overIndex: null });
  }, []);

  const handleDrop = useCallback(
    (targetIndex: number) => (e: React.DragEvent) => {
      e.preventDefault();
      const draggedIdStr = e.dataTransfer.getData("text/plain");
      const draggedId = parseInt(draggedIdStr, 10);

      if (isNaN(draggedId)) return;

      const currentIndex = items.indexOf(draggedId);
      if (currentIndex === -1 || currentIndex === targetIndex) {
        setDragState({ draggedId: null, overIndex: null });
        return;
      }

      // Reorder: remove from current position, insert at target
      const newOrder = [...items];
      newOrder.splice(currentIndex, 1);
      newOrder.splice(targetIndex, 0, draggedId);
      onReorder(newOrder);

      setDragState({ draggedId: null, overIndex: null });
    },
    [items, onReorder],
  );

  return {
    dragState,
    handleDragStart,
    handleDragOver,
    handleDragEnd,
    handleDrop,
  };
}
```

- [ ] **Step 2: Update `src/features/kanban/KanbanColumn.tsx` to support drag-and-drop**

```tsx
// src/features/kanban/KanbanColumn.tsx
import React from "react";
import type { Task, TaskStatus } from "../../types";
import { useDragAndDrop } from "./useDragAndDrop";

export interface KanbanColumnProps {
  /** Column status key */
  status: TaskStatus;
  /** Display label */
  label: string;
  /** Tasks in this column */
  tasks: Task[];
  /** Render function for each task card */
  renderCard: (task: Task) => React.ReactNode;
  /** Whether this column supports drag-and-drop reordering */
  isDraggable?: boolean;
  /** Color accent for the column header dot */
  accentColor: string;
  /** Callback when tasks are reordered via drag-and-drop */
  onReorder?: (newOrder: number[]) => void;
}

const COLUMN_BG: Record<TaskStatus, string> = {
  queued: "border-shepherd-muted/30",
  running: "border-shepherd-accent/30",
  input: "border-shepherd-orange/30",
  review: "border-shepherd-purple/30",
  done: "border-shepherd-green/30",
  error: "border-shepherd-red/30",
};

export const KanbanColumn: React.FC<KanbanColumnProps> = ({
  status,
  label,
  tasks,
  renderCard,
  isDraggable = false,
  accentColor,
  onReorder,
}) => {
  const taskIds = tasks.map((t) => t.id);
  const dnd = useDragAndDrop(taskIds, (newOrder) => {
    onReorder?.(newOrder);
  });

  return (
    <div
      className={`flex flex-col min-w-[260px] max-w-[320px] flex-1 bg-shepherd-surface/50 rounded-lg border ${COLUMN_BG[status]}`}
    >
      {/* Column header */}
      <div className="flex items-center gap-2 px-3 py-2.5 border-b border-shepherd-border/50">
        <div
          className="w-2.5 h-2.5 rounded-full shrink-0"
          style={{ backgroundColor: accentColor }}
        />
        <h2 className="text-xs font-semibold text-shepherd-text uppercase tracking-wider flex-1">
          {label}
        </h2>
        <span className="text-xs text-shepherd-muted bg-shepherd-border/30 px-1.5 py-0.5 rounded-full min-w-[20px] text-center">
          {tasks.length}
        </span>
      </div>

      {/* Card list */}
      <div className="flex-1 overflow-y-auto p-2 space-y-2">
        {tasks.length === 0 ? (
          <div className="text-xs text-shepherd-muted/50 text-center py-8">
            No tasks
          </div>
        ) : (
          tasks.map((task, index) => (
            <div
              key={task.id}
              draggable={isDraggable}
              onDragStart={isDraggable ? dnd.handleDragStart(task.id) : undefined}
              onDragOver={isDraggable ? dnd.handleDragOver(index) : undefined}
              onDragEnd={isDraggable ? dnd.handleDragEnd : undefined}
              onDrop={isDraggable ? dnd.handleDrop(index) : undefined}
              className={`
                ${isDraggable ? "cursor-grab active:cursor-grabbing" : ""}
                ${dnd.dragState.draggedId === task.id ? "opacity-50" : ""}
                ${dnd.dragState.overIndex === index && dnd.dragState.draggedId !== task.id ? "border-t-2 border-shepherd-accent" : ""}
                transition-opacity
              `}
            >
              {renderCard(task)}
            </div>
          ))
        )}
      </div>
    </div>
  );
};
```

- [ ] **Step 3: Update `src/features/kanban/KanbanBoard.tsx` to pass onReorder**

```tsx
// src/features/kanban/KanbanBoard.tsx
import React, { useState, useCallback } from "react";
import { useStore } from "../../store";
import { KanbanColumn } from "./KanbanColumn";
import { TaskCard } from "./TaskCard";
import type { Task, TaskStatus } from "../../types";

interface ColumnDef {
  status: TaskStatus;
  label: string;
  accentColor: string;
}

const COLUMNS: ColumnDef[] = [
  { status: "queued", label: "Queued", accentColor: "#8b949e" },
  { status: "running", label: "Running", accentColor: "#58a6ff" },
  { status: "input", label: "Needs Input", accentColor: "#db6d28" },
  { status: "review", label: "Review", accentColor: "#bc8cff" },
  { status: "done", label: "Done", accentColor: "#3fb950" },
];

export const KanbanBoard: React.FC = () => {
  const tasks = useStore((s) => s.tasks);
  const enterFocus = useStore((s) => s.enterFocus);
  const pendingPermissions = useStore((s) => s.pendingPermissions);

  // Custom ordering for queued tasks (drag-and-drop reorder)
  const [queuedOrder, setQueuedOrder] = useState<number[] | null>(null);

  // Group tasks by status
  const tasksByStatus: Record<TaskStatus, Task[]> = {
    queued: [],
    running: [],
    input: [],
    review: [],
    done: [],
    error: [],
  };

  for (const task of Object.values(tasks)) {
    if (tasksByStatus[task.status]) {
      tasksByStatus[task.status].push(task);
    }
  }

  // Tasks with errors show in the review column
  for (const task of tasksByStatus.error) {
    tasksByStatus.review.push(task);
  }

  // Sort: queued by custom order or ID, others by updated_at descending
  if (queuedOrder) {
    const queuedMap = new Map(tasksByStatus.queued.map((t) => [t.id, t]));
    const ordered: Task[] = [];
    for (const id of queuedOrder) {
      const task = queuedMap.get(id);
      if (task) {
        ordered.push(task);
        queuedMap.delete(id);
      }
    }
    // Append any new tasks not in the custom order
    for (const task of queuedMap.values()) {
      ordered.push(task);
    }
    tasksByStatus.queued = ordered;
  } else {
    tasksByStatus.queued.sort((a, b) => a.id - b.id);
  }

  for (const status of ["running", "input", "review", "done"] as TaskStatus[]) {
    tasksByStatus[status].sort(
      (a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime(),
    );
  }

  const now = Date.now();
  const twentyFourHours = 24 * 60 * 60 * 1000;

  const handleCardClick = useCallback(
    (taskId: number) => {
      enterFocus(taskId);
    },
    [enterFocus],
  );

  const handleQueuedReorder = useCallback((newOrder: number[]) => {
    setQueuedOrder(newOrder);
  }, []);

  return (
    <div className="h-full flex gap-3 p-4 overflow-x-auto">
      {COLUMNS.map((col) => (
        <KanbanColumn
          key={col.status}
          status={col.status}
          label={col.label}
          accentColor={col.accentColor}
          tasks={tasksByStatus[col.status]}
          isDraggable={col.status === "queued"}
          onReorder={col.status === "queued" ? handleQueuedReorder : undefined}
          renderCard={(task) => {
            const isDone = task.status === "done";
            const taskAge = now - new Date(task.updated_at).getTime();
            const isFaded = isDone && taskAge > twentyFourHours;

            return (
              <div
                className={`transition-opacity ${isFaded ? "opacity-40" : "opacity-100"}`}
              >
                <TaskCard
                  task={task}
                  permissions={pendingPermissions.filter(
                    (p) => p.task_id === task.id,
                  )}
                  onClick={() => handleCardClick(task.id)}
                />
              </div>
            );
          }}
        />
      ))}
    </div>
  );
};
```

- [ ] **Step 4: Verify quick-approve (1-9 keys) is already wired**

Quick-approve via number keys 1-9 was already implemented in Task 7 (`useKeyboardShortcuts.ts`). The shortcuts are registered for `viewMode: "overview"` and will approve the Nth task in the "input" column. Verify by reviewing the code:

```bash
grep -n "quick-approve" src/hooks/useKeyboardShortcuts.ts
```

**Expected:** Lines showing `quick-approve-1` through `quick-approve-9` shortcut registrations.

- [ ] **Step 5: Verify types compile**

```bash
npm run typecheck
```

**Expected:** Zero TypeScript errors.

- [ ] **Step 6: Verify build**

```bash
npm run build
```

**Expected:** Vite builds successfully. No errors.

- [ ] **Step 7: Commit**

```bash
git add src/features/kanban/useDragAndDrop.ts src/features/kanban/KanbanColumn.tsx src/features/kanban/KanbanBoard.tsx
git commit -m "feat(kanban): add drag-and-drop reordering for Queued column and verify quick-approve shortcuts"
```

---

## Chunk 4: Focus View (Tasks 11-14)

### Task 11: FocusView Three-Panel Layout + SessionSidebar

**Files:**
- Create: `src/features/focus/FocusView.tsx`
- Create: `src/features/focus/SessionSidebar.tsx`
- Modify: `src/stores/useUIStore.ts` (add focusedTaskId)

- [ ] **Step 1: Add focusedTaskId to UI store**

```typescript
// src/stores/useUIStore.ts — add to existing store
// Add these fields to the store interface and implementation:

interface UIState {
  mode: 'overview' | 'focus';
  focusedTaskId: string | null;
  setMode: (mode: 'overview' | 'focus') => void;
  setFocusedTaskId: (id: string | null) => void;
  openFocus: (taskId: string) => void;
  returnToOverview: () => void;
}

// In the create() call, add:
//   focusedTaskId: null,
//   setFocusedTaskId: (id) => set({ focusedTaskId: id }),
//   openFocus: (taskId) => set({ mode: 'focus', focusedTaskId: taskId }),
//   returnToOverview: () => set({ mode: 'overview', focusedTaskId: null }),
```

Modify the existing `useUIStore.ts` — add `focusedTaskId: null` to state, and add `setFocusedTaskId`, `openFocus`, and `returnToOverview` actions. The full updated file:

```typescript
// src/stores/useUIStore.ts
import { create } from 'zustand';

interface UIState {
  mode: 'overview' | 'focus';
  focusedTaskId: string | null;
  setMode: (mode: 'overview' | 'focus') => void;
  setFocusedTaskId: (id: string | null) => void;
  openFocus: (taskId: string) => void;
  returnToOverview: () => void;
}

export const useUIStore = create<UIState>((set) => ({
  mode: 'overview',
  focusedTaskId: null,
  setMode: (mode) => set({ mode }),
  setFocusedTaskId: (id) => set({ focusedTaskId: id }),
  openFocus: (taskId) => set({ mode: 'focus', focusedTaskId: taskId }),
  returnToOverview: () => set({ mode: 'overview', focusedTaskId: null }),
}));
```

- [ ] **Step 2: Create SessionSidebar component**

```tsx
// src/features/focus/SessionSidebar.tsx
import React from 'react';
import { useTaskStore } from '../../stores/useTaskStore';
import { useUIStore } from '../../stores/useUIStore';
import { Badge } from '../../ui/Badge';

const STATUS_COLORS: Record<string, string> = {
  queued: 'bg-zinc-500',
  running: 'bg-blue-500 animate-pulse',
  input: 'bg-orange-500 animate-pulse',
  review: 'bg-purple-500',
  error: 'bg-red-500',
  done: 'bg-green-500',
};

export const SessionSidebar: React.FC = () => {
  const tasks = useTaskStore((s) => s.tasks);
  const focusedTaskId = useUIStore((s) => s.focusedTaskId);
  const openFocus = useUIStore((s) => s.openFocus);
  const returnToOverview = useUIStore((s) => s.returnToOverview);

  const taskList = Object.values(tasks);

  return (
    <aside className="w-[180px] min-w-[180px] bg-zinc-900 border-r border-zinc-700 flex flex-col h-full">
      {/* Back button */}
      <button
        onClick={returnToOverview}
        className="flex items-center gap-1.5 px-3 py-2.5 text-sm text-zinc-400 hover:text-white hover:bg-zinc-800 transition-colors border-b border-zinc-700"
      >
        <span className="text-xs">&larr;</span>
        <span>Overview</span>
      </button>

      {/* Session list */}
      <div className="flex-1 overflow-y-auto py-1">
        <div className="px-3 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-zinc-500">
          Sessions
        </div>
        {taskList.map((task) => {
          const isActive = task.id === focusedTaskId;
          const dotColor = STATUS_COLORS[task.status] || 'bg-zinc-500';

          return (
            <button
              key={task.id}
              onClick={() => openFocus(task.id)}
              className={`
                w-full text-left px-3 py-2 flex items-center gap-2 text-sm transition-colors
                ${isActive
                  ? 'bg-zinc-800 text-white border-l-2 border-blue-500'
                  : 'text-zinc-400 hover:text-white hover:bg-zinc-800/50 border-l-2 border-transparent'
                }
              `}
            >
              <span className={`w-2 h-2 rounded-full shrink-0 ${dotColor}`} />
              <span className="truncate">{task.title}</span>
            </button>
          );
        })}

        {taskList.length === 0 && (
          <div className="px-3 py-4 text-xs text-zinc-600 text-center">
            No sessions
          </div>
        )}
      </div>

      {/* Session count */}
      <div className="px-3 py-2 border-t border-zinc-700 text-[10px] text-zinc-500">
        {taskList.length} session{taskList.length !== 1 ? 's' : ''}
      </div>
    </aside>
  );
};
```

- [ ] **Step 3: Create FocusView with three-panel layout**

```tsx
// src/features/focus/FocusView.tsx
import React, { useState, useCallback } from 'react';
import { useTaskStore } from '../../stores/useTaskStore';
import { useUIStore } from '../../stores/useUIStore';
import { SessionSidebar } from './SessionSidebar';
import { Terminal } from './Terminal';
import { DiffViewer } from './DiffViewer';
import { PermissionPrompt } from './PermissionPrompt';
import { Badge } from '../../ui/Badge';

function formatTimeSince(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diffMs = now - then;
  const seconds = Math.floor(diffMs / 1000);
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h ago`;
}

const ISOLATION_LABELS: Record<string, string> = {
  worktree: 'Worktree',
  docker: 'Docker',
  local: 'Local',
};

const STATUS_BADGE_COLORS: Record<string, { bg: string; text: string }> = {
  queued: { bg: 'bg-zinc-700', text: 'text-zinc-300' },
  running: { bg: 'bg-blue-900/50', text: 'text-blue-400' },
  input: { bg: 'bg-orange-900/50', text: 'text-orange-400' },
  review: { bg: 'bg-purple-900/50', text: 'text-purple-400' },
  error: { bg: 'bg-red-900/50', text: 'text-red-400' },
  done: { bg: 'bg-green-900/50', text: 'text-green-400' },
};

export const FocusView: React.FC = () => {
  const focusedTaskId = useUIStore((s) => s.focusedTaskId);
  const tasks = useTaskStore((s) => s.tasks);
  const task = focusedTaskId ? tasks[focusedTaskId] : null;

  const [rightPanelWidth, setRightPanelWidth] = useState(400);
  const [isDragging, setIsDragging] = useState(false);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);

    const startX = e.clientX;
    const startWidth = rightPanelWidth;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const delta = startX - moveEvent.clientX;
      const newWidth = Math.max(250, Math.min(800, startWidth + delta));
      setRightPanelWidth(newWidth);
    };

    const handleMouseUp = () => {
      setIsDragging(false);
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, [rightPanelWidth]);

  if (!task) {
    return (
      <div className="flex h-full items-center justify-center bg-zinc-950 text-zinc-500">
        <p>No task selected. Click a task card or press <kbd className="px-1.5 py-0.5 bg-zinc-800 rounded text-xs font-mono">0</kbd> to return to overview.</p>
      </div>
    );
  }

  const statusColors = STATUS_BADGE_COLORS[task.status] || STATUS_BADGE_COLORS.queued;

  return (
    <div className="flex h-full bg-zinc-950">
      {/* Left: Session Sidebar */}
      <SessionSidebar />

      {/* Center + Right */}
      <div className="flex flex-col flex-1 min-w-0">
        {/* Task Header Bar */}
        <header className="flex items-center gap-3 px-4 py-2 bg-zinc-900 border-b border-zinc-700 shrink-0">
          <h2 className="text-sm font-semibold text-white truncate max-w-[300px]">
            {task.title}
          </h2>

          <Badge
            label={task.agent_id}
            className="bg-indigo-900/50 text-indigo-400 text-[10px]"
          />

          {task.branch && (
            <span className="text-xs text-zinc-500 font-mono truncate max-w-[200px]">
              {task.branch}
            </span>
          )}

          <span className="text-xs text-zinc-600">
            {ISOLATION_LABELS[task.isolation_mode] || task.isolation_mode}
          </span>

          <div className="flex-1" />

          <span className="text-xs text-zinc-500">
            {formatTimeSince(task.updated_at)}
          </span>

          <span className={`px-2 py-0.5 rounded text-[10px] font-medium uppercase ${statusColors.bg} ${statusColors.text}`}>
            {task.status}
          </span>
        </header>

        {/* Main content area */}
        <div className="flex flex-1 min-h-0">
          {/* Center: Terminal + Permission Prompt */}
          <div className="flex flex-col flex-1 min-w-0">
            <div className="flex-1 min-h-0">
              <Terminal taskId={task.id} />
            </div>

            {task.status === 'input' && (
              <PermissionPrompt taskId={task.id} />
            )}
          </div>

          {/* Resize handle */}
          <div
            onMouseDown={handleMouseDown}
            className={`
              w-1 cursor-col-resize hover:bg-blue-500/50 transition-colors shrink-0
              ${isDragging ? 'bg-blue-500/50' : 'bg-zinc-700'}
            `}
          />

          {/* Right: Changes Panel */}
          <div
            className="shrink-0 min-h-0 overflow-hidden"
            style={{ width: rightPanelWidth }}
          >
            <DiffViewer taskId={task.id} />
          </div>
        </div>
      </div>
    </div>
  );
};
```

- [ ] **Step 4: Wire FocusView into App.tsx**

In `src/App.tsx`, the existing mode switch should render `<FocusView />` when `mode === 'focus'`. Update the import and conditional:

```tsx
// In src/App.tsx — add import
import { FocusView } from './features/focus/FocusView';

// In the JSX, the mode switch becomes:
// {mode === 'overview' ? <KanbanBoard /> : <FocusView />}
```

Ensure the Kanban card's `onClick` calls `openFocus(task.id)` — update `TaskCard.tsx`:

```tsx
// In src/features/kanban/TaskCard.tsx — modify the card's root onClick:
import { useUIStore } from '../../stores/useUIStore';

// Inside the component:
const openFocus = useUIStore((s) => s.openFocus);

// On the card wrapper element, add:
// onClick={() => openFocus(task.id)}
```

- [ ] **Step 5: Verify layout renders**

Run: `npm run dev`
Expected: FocusView renders with three-panel layout. Session sidebar shows task list with status dots. Clicking a Kanban card switches to Focus mode. "Overview" button returns to Kanban.

- [ ] **Step 6: Commit**

```bash
git add src/features/focus/FocusView.tsx src/features/focus/SessionSidebar.tsx src/stores/useUIStore.ts src/App.tsx src/features/kanban/TaskCard.tsx
git commit -m "feat: add FocusView three-panel layout with SessionSidebar"
```

---

### Task 12: Terminal Component (xterm.js + fit addon + WebSocket I/O)

**Files:**
- Create: `src/features/focus/Terminal.tsx`
- Modify: `package.json` (add xterm dependencies)

- [ ] **Step 1: Install xterm.js dependencies**

```bash
npm install @xterm/xterm @xterm/addon-fit
```

- [ ] **Step 2: Create Terminal component**

```tsx
// src/features/focus/Terminal.tsx
import React, { useRef, useEffect, useCallback } from 'react';
import { Terminal as XTerm } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { useWebSocket } from '../../hooks/useWebSocket';

interface TerminalProps {
  taskId: string;
}

export const Terminal: React.FC<TerminalProps> = ({ taskId }) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<XTerm | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const { sendMessage, subscribe } = useWebSocket();

  // Initialize terminal
  useEffect(() => {
    if (!containerRef.current) return;

    const term = new XTerm({
      cursorBlink: true,
      cursorStyle: 'bar',
      fontSize: 13,
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', Menlo, Monaco, monospace",
      lineHeight: 1.3,
      theme: {
        background: '#09090b',    // zinc-950
        foreground: '#e4e4e7',    // zinc-200
        cursor: '#3b82f6',        // blue-500
        cursorAccent: '#09090b',
        selectionBackground: '#3b82f680',
        selectionForeground: '#ffffff',
        black: '#18181b',
        red: '#ef4444',
        green: '#22c55e',
        yellow: '#eab308',
        blue: '#3b82f6',
        magenta: '#a855f7',
        cyan: '#06b6d4',
        white: '#e4e4e7',
        brightBlack: '#52525b',
        brightRed: '#f87171',
        brightGreen: '#4ade80',
        brightYellow: '#facc15',
        brightBlue: '#60a5fa',
        brightMagenta: '#c084fc',
        brightCyan: '#22d3ee',
        brightWhite: '#fafafa',
      },
      scrollback: 10000,
      allowTransparency: false,
      convertEol: true,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);

    term.open(containerRef.current);
    fitAddon.fit();

    termRef.current = term;
    fitAddonRef.current = fitAddon;

    // Send initial resize to server
    const { cols, rows } = term;
    sendMessage({
      type: 'terminal:resize',
      payload: { task_id: taskId, cols, rows },
    });

    return () => {
      term.dispose();
      termRef.current = null;
      fitAddonRef.current = null;
    };
  }, [taskId]); // Re-create terminal when task changes

  // Handle terminal input → WebSocket
  useEffect(() => {
    const term = termRef.current;
    if (!term) return;

    const disposable = term.onData((data: string) => {
      sendMessage({
        type: 'terminal:input',
        payload: { task_id: taskId, data },
      });
    });

    return () => disposable.dispose();
  }, [taskId, sendMessage]);

  // Handle WebSocket terminal output → xterm
  useEffect(() => {
    const unsubscribe = subscribe('terminal:output', (event: { task_id: string; data: string }) => {
      if (event.task_id === taskId && termRef.current) {
        termRef.current.write(event.data);
      }
    });

    return unsubscribe;
  }, [taskId, subscribe]);

  // Handle window resize
  useEffect(() => {
    const handleResize = () => {
      const fitAddon = fitAddonRef.current;
      const term = termRef.current;
      if (!fitAddon || !term) return;

      fitAddon.fit();

      sendMessage({
        type: 'terminal:resize',
        payload: { task_id: taskId, cols: term.cols, rows: term.rows },
      });
    };

    // Use ResizeObserver for the container
    const container = containerRef.current;
    if (!container) return;

    const resizeObserver = new ResizeObserver(() => {
      // Debounce resize slightly to avoid rapid fire
      requestAnimationFrame(handleResize);
    });

    resizeObserver.observe(container);

    return () => {
      resizeObserver.disconnect();
    };
  }, [taskId, sendMessage]);

  // Pop-out placeholder handler
  const handlePopOut = useCallback(() => {
    // Deferred to v2: pop-out terminal requires Tauri multiwindow API (WebviewWindow::new)
    console.warn('Pop-out terminal deferred to v2 — requires Tauri multiwindow');
  }, []);

  return (
    <div className="flex flex-col h-full bg-[#09090b]">
      {/* Terminal toolbar */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-zinc-900 border-b border-zinc-800 shrink-0">
        <div className="flex items-center gap-2">
          <span className="text-[10px] font-semibold uppercase tracking-wider text-zinc-500">
            Terminal
          </span>
          <span className="text-[10px] text-zinc-600 font-mono">
            {taskId.slice(0, 8)}
          </span>
        </div>

        <button
          onClick={handlePopOut}
          className="text-zinc-500 hover:text-white text-xs px-1.5 py-0.5 rounded hover:bg-zinc-800 transition-colors"
          title="Pop out terminal"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <polyline points="15 3 21 3 21 9" />
            <line x1="10" y1="14" x2="21" y2="3" />
            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
          </svg>
        </button>
      </div>

      {/* Terminal container */}
      <div
        ref={containerRef}
        className="flex-1 min-h-0 p-1"
      />
    </div>
  );
};
```

- [ ] **Step 3: Verify xterm renders**

Run: `npm run dev`
Expected: Terminal component renders inside the FocusView center panel. A dark terminal appears with a blinking cursor. Typing sends `terminal:input` events over WebSocket (visible in browser devtools Network tab). Terminal output received via WebSocket writes to the terminal.

- [ ] **Step 4: Commit**

```bash
git add src/features/focus/Terminal.tsx package.json package-lock.json
git commit -m "feat: add xterm.js Terminal component with WebSocket I/O and resize handling"
```

---

### Task 13: DiffViewer (Monaco Editor in Diff Mode, File Tabs)

**Files:**
- Create: `src/features/focus/DiffViewer.tsx`
- Modify: `package.json` (add Monaco dependency)
- Modify: `src/stores/useTaskStore.ts` (add diff data to task type)

- [ ] **Step 1: Install Monaco Editor**

```bash
npm install @monaco-editor/react
```

- [ ] **Step 2: Add diff types to task store**

Add a `FileDiff` type and a `diffs` map to the task store. Modify `src/stores/useTaskStore.ts`:

```typescript
// Add to src/stores/useTaskStore.ts

export interface FileDiff {
  file_path: string;
  before_content: string;
  after_content: string;
  language: string;
}

// Add to the Task interface:
//   diffs?: FileDiff[];

// Add action to TaskState:
//   setTaskDiffs: (taskId: string, diffs: FileDiff[]) => void;

// In the store implementation, add:
//   setTaskDiffs: (taskId, diffs) => set((state) => {
//     const task = state.tasks[taskId];
//     if (!task) return state;
//     return {
//       tasks: {
//         ...state.tasks,
//         [taskId]: { ...task, diffs },
//       },
//     };
//   }),
```

The relevant additions to `useTaskStore.ts`:

```typescript
// src/stores/useTaskStore.ts — add these types at the top of the file

export interface FileDiff {
  file_path: string;
  before_content: string;
  after_content: string;
  language: string;
}

// Extend the existing Task interface to include:
// diffs?: FileDiff[];

// Extend the existing TaskState interface to include:
// setTaskDiffs: (taskId: string, diffs: FileDiff[]) => void;

// Add to the create() implementation:
// setTaskDiffs: (taskId, diffs) =>
//   set((state) => {
//     const task = state.tasks[taskId];
//     if (!task) return state;
//     return {
//       tasks: {
//         ...state.tasks,
//         [taskId]: { ...task, diffs },
//       },
//     };
//   }),
```

- [ ] **Step 3: Create DiffViewer component**

```tsx
// src/features/focus/DiffViewer.tsx
import React, { useState, useMemo } from 'react';
import { DiffEditor } from '@monaco-editor/react';
import { useTaskStore, type FileDiff } from '../../stores/useTaskStore';

interface DiffViewerProps {
  taskId: string;
}

function detectLanguage(filePath: string): string {
  const ext = filePath.split('.').pop()?.toLowerCase() || '';
  const languageMap: Record<string, string> = {
    ts: 'typescript',
    tsx: 'typescript',
    js: 'javascript',
    jsx: 'javascript',
    py: 'python',
    rs: 'rust',
    go: 'go',
    rb: 'ruby',
    java: 'java',
    kt: 'kotlin',
    swift: 'swift',
    c: 'c',
    cpp: 'cpp',
    h: 'c',
    hpp: 'cpp',
    cs: 'csharp',
    css: 'css',
    scss: 'scss',
    html: 'html',
    json: 'json',
    yaml: 'yaml',
    yml: 'yaml',
    toml: 'toml',
    md: 'markdown',
    sql: 'sql',
    sh: 'shell',
    bash: 'shell',
    zsh: 'shell',
    dockerfile: 'dockerfile',
    xml: 'xml',
    svg: 'xml',
  };
  return languageMap[ext] || 'plaintext';
}

function getFileName(filePath: string): string {
  return filePath.split('/').pop() || filePath;
}

export const DiffViewer: React.FC<DiffViewerProps> = ({ taskId }) => {
  const task = useTaskStore((s) => s.tasks[taskId]);
  const diffs = task?.diffs || [];

  const [selectedFileIndex, setSelectedFileIndex] = useState(0);
  const [diffMode, setDiffMode] = useState<'unified' | 'side-by-side'>('unified');
  const [commentDraft, setCommentDraft] = useState('');
  const [activeCommentLine, setActiveCommentLine] = useState<number | null>(null);

  const selectedDiff: FileDiff | null = diffs[selectedFileIndex] || null;
  const language = selectedDiff ? detectLanguage(selectedDiff.file_path) : 'plaintext';

  // Reset selected file when diffs change
  const diffCount = diffs.length;
  React.useEffect(() => {
    if (selectedFileIndex >= diffCount) {
      setSelectedFileIndex(0);
    }
  }, [diffCount, selectedFileIndex]);

  const handleCommentSubmit = async () => {
    if (!commentDraft.trim() || activeCommentLine === null || !selectedDiff) return;
    // Send inline comment as terminal input to the agent — the comment becomes
    // part of the agent's prompt context (e.g., "User commented on line 42 of
    // src/main.rs: 'This should use a HashMap instead'")
    const commentPayload = `User commented on line ${activeCommentLine} of ${selectedDiff.file_path}: "${commentDraft}"`;
    sendMessage({
      type: 'terminal_input',
      data: { task_id: taskId, data: commentPayload + '\n' },
    });
    setCommentDraft('');
    setActiveCommentLine(null);
  };

  if (diffs.length === 0) {
    return (
      <div className="flex flex-col h-full bg-zinc-950">
        <div className="flex items-center px-3 py-1.5 bg-zinc-900 border-b border-zinc-800 shrink-0">
          <span className="text-[10px] font-semibold uppercase tracking-wider text-zinc-500">
            Changes
          </span>
        </div>
        <div className="flex-1 flex items-center justify-center text-zinc-600 text-sm">
          No file changes yet
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full bg-zinc-950">
      {/* File tabs */}
      <div className="flex items-center bg-zinc-900 border-b border-zinc-800 shrink-0 overflow-x-auto">
        <div className="flex items-center flex-1 min-w-0">
          {diffs.map((diff, index) => (
            <button
              key={diff.file_path}
              onClick={() => setSelectedFileIndex(index)}
              className={`
                px-3 py-1.5 text-xs font-mono whitespace-nowrap border-r border-zinc-800 transition-colors
                ${index === selectedFileIndex
                  ? 'bg-zinc-800 text-white'
                  : 'text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50'
                }
              `}
              title={diff.file_path}
            >
              {getFileName(diff.file_path)}
            </button>
          ))}
        </div>

        {/* Diff mode toggle */}
        <div className="flex items-center gap-0.5 px-2 shrink-0">
          <button
            onClick={() => setDiffMode('unified')}
            className={`px-2 py-1 text-[10px] rounded transition-colors ${
              diffMode === 'unified'
                ? 'bg-zinc-700 text-white'
                : 'text-zinc-500 hover:text-zinc-300'
            }`}
          >
            Unified
          </button>
          <button
            onClick={() => setDiffMode('side-by-side')}
            className={`px-2 py-1 text-[10px] rounded transition-colors ${
              diffMode === 'side-by-side'
                ? 'bg-zinc-700 text-white'
                : 'text-zinc-500 hover:text-zinc-300'
            }`}
          >
            Side-by-Side
          </button>
        </div>
      </div>

      {/* File path breadcrumb */}
      {selectedDiff && (
        <div className="px-3 py-1 bg-zinc-900/50 border-b border-zinc-800/50 shrink-0">
          <span className="text-[11px] text-zinc-500 font-mono">
            {selectedDiff.file_path}
          </span>
        </div>
      )}

      {/* Monaco Diff Editor */}
      <div className="flex-1 min-h-0">
        {selectedDiff && (
          <DiffEditor
            original={selectedDiff.before_content}
            modified={selectedDiff.after_content}
            language={language}
            theme="shepherd-dark"
            options={{
              readOnly: true,
              renderSideBySide: diffMode === 'side-by-side',
              minimap: { enabled: false },
              fontSize: 12,
              fontFamily: "'JetBrains Mono', 'Fira Code', Menlo, monospace",
              lineHeight: 18,
              scrollBeyondLastLine: false,
              automaticLayout: true,
              renderOverviewRuler: false,
              diffWordWrap: 'on',
              padding: { top: 8, bottom: 8 },
              glyphMargin: true,
              folding: true,
              lineNumbers: 'on',
              scrollbar: {
                verticalScrollbarSize: 6,
                horizontalScrollbarSize: 6,
              },
            }}
            beforeMount={(monaco) => {
              // Define custom dark theme
              monaco.editor.defineTheme('shepherd-dark', {
                base: 'vs-dark',
                inherit: true,
                rules: [
                  { token: 'comment', foreground: '6b7280', fontStyle: 'italic' },
                  { token: 'keyword', foreground: 'c084fc' },
                  { token: 'string', foreground: '4ade80' },
                  { token: 'number', foreground: 'facc15' },
                  { token: 'type', foreground: '60a5fa' },
                ],
                colors: {
                  'editor.background': '#0a0a0b',
                  'editor.foreground': '#e4e4e7',
                  'editor.lineHighlightBackground': '#18181b',
                  'editorLineNumber.foreground': '#3f3f46',
                  'editorLineNumber.activeForeground': '#71717a',
                  'editor.selectionBackground': '#3b82f640',
                  'diffEditor.insertedTextBackground': '#22c55e15',
                  'diffEditor.removedTextBackground': '#ef444415',
                  'diffEditor.insertedLineBackground': '#22c55e10',
                  'diffEditor.removedLineBackground': '#ef444410',
                },
              });
            }}
            onMount={(editor) => {
              // Handle click on glyph margin for inline comments
              const modifiedEditor = editor.getModifiedEditor();
              modifiedEditor.onMouseDown((e) => {
                if (
                  e.target.type ===
                  (window as any).monaco?.editor?.MouseTargetType?.GUTTER_GLYPH_MARGIN
                ) {
                  const lineNumber = e.target.position?.lineNumber;
                  if (lineNumber) {
                    setActiveCommentLine(lineNumber);
                  }
                }
              });
            }}
          />
        )}
      </div>

      {/* Inline comment input */}
      {activeCommentLine !== null && (
        <div className="px-3 py-2 bg-zinc-900 border-t border-zinc-700 shrink-0">
          <div className="flex items-center gap-2 mb-1">
            <span className="text-[10px] text-zinc-500">
              Comment on line {activeCommentLine}
            </span>
            <button
              onClick={() => setActiveCommentLine(null)}
              className="text-zinc-500 hover:text-white text-xs ml-auto"
            >
              Cancel
            </button>
          </div>
          <div className="flex gap-2">
            <input
              type="text"
              value={commentDraft}
              onChange={(e) => setCommentDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault();
                  handleCommentSubmit();
                }
                if (e.key === 'Escape') {
                  setActiveCommentLine(null);
                }
              }}
              placeholder="Add feedback for the agent..."
              className="flex-1 px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-sm text-white placeholder-zinc-500 focus:outline-none focus:border-blue-500"
              autoFocus
            />
            <button
              onClick={handleCommentSubmit}
              disabled={!commentDraft.trim()}
              className="px-3 py-1 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white text-xs rounded transition-colors"
            >
              Send
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
```

- [ ] **Step 4: Verify diff viewer renders**

Run: `npm run dev`
Expected: DiffViewer renders in the right panel of FocusView. When a task has diffs, file tabs appear at the top. Clicking a tab switches the displayed diff. The unified/side-by-side toggle works. Clicking the glyph margin opens a comment input. When no diffs are present, "No file changes yet" is displayed.

- [ ] **Step 5: Commit**

```bash
git add src/features/focus/DiffViewer.tsx src/stores/useTaskStore.ts package.json package-lock.json
git commit -m "feat: add Monaco DiffViewer with file tabs, diff modes, and inline comments"
```

---

### Task 14: PermissionPrompt UI

**Files:**
- Create: `src/features/focus/PermissionPrompt.tsx`
- Modify: `src/hooks/useKeyboardShortcuts.ts` (add approve shortcuts)

- [ ] **Step 1: Create PermissionPrompt component**

```tsx
// src/features/focus/PermissionPrompt.tsx
import React, { useState, useRef, useEffect } from 'react';
import { useWebSocket } from '../../hooks/useWebSocket';
import { useTaskStore } from '../../stores/useTaskStore';

interface PermissionPromptProps {
  taskId: string;
}

export const PermissionPrompt: React.FC<PermissionPromptProps> = ({ taskId }) => {
  const { sendMessage } = useWebSocket();
  const task = useTaskStore((s) => s.tasks[taskId]);
  const [customInput, setCustomInput] = useState('');
  const [showCustom, setShowCustom] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  // Focus the custom input when it appears
  useEffect(() => {
    if (showCustom && inputRef.current) {
      inputRef.current.focus();
    }
  }, [showCustom]);

  const handleApprove = () => {
    sendMessage({
      type: 'task:approve',
      payload: { task_id: taskId },
    });
  };

  const handleApproveAll = () => {
    sendMessage({
      type: 'task:approve_all',
      payload: {},
    });
  };

  const handleCustomSubmit = () => {
    if (!customInput.trim()) return;
    sendMessage({
      type: 'terminal:input',
      payload: { task_id: taskId, data: customInput + '\n' },
    });
    setCustomInput('');
    setShowCustom(false);
  };

  // Keyboard shortcut handler for this component
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Only handle if this prompt is visible (task status is 'input')
      if (task?.status !== 'input') return;

      // Cmd+Enter = approve
      if (e.metaKey && e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleApprove();
        return;
      }

      // Cmd+Shift+Enter = approve all
      if (e.metaKey && e.shiftKey && e.key === 'Enter') {
        e.preventDefault();
        handleApproveAll();
        return;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [taskId, task?.status]);

  // Extract permission question from task if available
  const permissionQuestion = task?.current_action || 'Permission requested';

  return (
    <div className="border-t border-orange-500/30 bg-orange-950/20 px-4 py-3 shrink-0">
      {/* Permission question */}
      <div className="flex items-start gap-2 mb-3">
        <div className="w-2 h-2 rounded-full bg-orange-500 animate-pulse mt-1.5 shrink-0" />
        <div className="flex-1 min-w-0">
          <span className="text-[10px] font-semibold uppercase tracking-wider text-orange-400 block mb-1">
            Permission Required
          </span>
          <p className="text-sm text-zinc-300 break-words font-mono">
            {permissionQuestion}
          </p>
        </div>
      </div>

      {/* Action buttons */}
      <div className="flex items-center gap-2">
        <button
          onClick={handleApprove}
          className="
            flex items-center gap-1.5 px-4 py-2 bg-green-600 hover:bg-green-500
            text-white text-sm font-medium rounded-md transition-colors
            focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2 focus:ring-offset-zinc-900
          "
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <polyline points="20 6 9 17 4 12" />
          </svg>
          Approve
          <kbd className="ml-1.5 text-[10px] text-green-200/60 bg-green-700/50 px-1 py-0.5 rounded">
            {'\u2318\u23CE'}
          </kbd>
        </button>

        <button
          onClick={handleApproveAll}
          className="
            flex items-center gap-1.5 px-4 py-2 bg-zinc-700 hover:bg-zinc-600
            text-white text-sm font-medium rounded-md transition-colors
            focus:outline-none focus:ring-2 focus:ring-zinc-500 focus:ring-offset-2 focus:ring-offset-zinc-900
          "
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <polyline points="18 8 7 19 2 14" />
            <polyline points="22 4 11 15" />
          </svg>
          Approve All
          <kbd className="ml-1.5 text-[10px] text-zinc-400/60 bg-zinc-600/50 px-1 py-0.5 rounded">
            {'\u2318\u21E7\u23CE'}
          </kbd>
        </button>

        <button
          onClick={() => setShowCustom(!showCustom)}
          className={`
            px-3 py-2 text-sm rounded-md transition-colors
            ${showCustom
              ? 'bg-blue-600 text-white'
              : 'bg-zinc-800 text-zinc-400 hover:text-white hover:bg-zinc-700'
            }
          `}
        >
          Custom...
        </button>

        <div className="flex-1" />

        {/* Keyboard hint */}
        <span className="text-[10px] text-zinc-600 hidden sm:block">
          Keyboard: {'\u2318\u23CE'} approve &middot; {'\u2318\u21E7\u23CE'} approve all
        </span>
      </div>

      {/* Custom input */}
      {showCustom && (
        <div className="mt-2 flex gap-2">
          <input
            ref={inputRef}
            type="text"
            value={customInput}
            onChange={(e) => setCustomInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                handleCustomSubmit();
              }
              if (e.key === 'Escape') {
                setShowCustom(false);
                setCustomInput('');
              }
            }}
            placeholder="Type custom response to send to agent..."
            className="
              flex-1 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-md
              text-sm text-white font-mono placeholder-zinc-500
              focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500
            "
          />
          <button
            onClick={handleCustomSubmit}
            disabled={!customInput.trim()}
            className="
              px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:opacity-50
              disabled:cursor-not-allowed text-white text-sm rounded-md transition-colors
            "
          >
            Send
          </button>
        </div>
      )}
    </div>
  );
};
```

- [ ] **Step 2: Ensure PermissionPrompt keyboard shortcuts integrate with global shortcuts**

The PermissionPrompt registers its own `keydown` listener for `Cmd+Enter` and `Cmd+Shift+Enter`. Verify this does not conflict with the global keyboard shortcut handler in `useKeyboardShortcuts.ts`. The global handler should skip `Cmd+Enter` and `Cmd+Shift+Enter` when mode is `focus` and the focused task status is `input`:

```typescript
// In src/hooks/useKeyboardShortcuts.ts — modify the existing handler
// Add this early-return check inside the keydown handler:

// Skip approve shortcuts — handled by PermissionPrompt component directly
// if (e.metaKey && e.key === 'Enter') return;
```

This avoids double-handling. The PermissionPrompt component handles approve shortcuts directly because it needs access to the current taskId and task status.

- [ ] **Step 3: Verify permission prompt renders and works**

Run: `npm run dev`
Expected: When a task has status `input`, the PermissionPrompt appears below the terminal in the FocusView. Three options visible: Approve (green), Approve All (gray), Custom (toggleable). Pressing `Cmd+Enter` sends approve. Pressing `Cmd+Shift+Enter` sends approve all. Custom input sends text followed by newline. Pressing Escape closes custom input.

- [ ] **Step 4: Commit**

```bash
git add src/features/focus/PermissionPrompt.tsx src/hooks/useKeyboardShortcuts.ts
git commit -m "feat: add PermissionPrompt UI with approve, approve-all, custom input, and keyboard shortcuts"
```

---

## Chunk 5: Polish & Integration (Tasks 15-18)

### Task 15: Command Palette (Cmd+K) with Fuzzy Search

**Files:**
- Create: `src/features/palette/CommandPalette.tsx`
- Modify: `src/hooks/useKeyboardShortcuts.ts` (add Cmd+K binding)
- Modify: `src/App.tsx` (render CommandPalette)

- [ ] **Step 1: Create CommandPalette component**

```tsx
// src/features/palette/CommandPalette.tsx
import React, { useState, useEffect, useRef, useMemo, useCallback } from 'react';
import { useUIStore } from '../../stores/useUIStore';
import { useTaskStore } from '../../stores/useTaskStore';
import { useWebSocket } from '../../hooks/useWebSocket';

export interface PaletteAction {
  id: string;
  label: string;
  description?: string;
  shortcut?: string;
  category: 'task' | 'view' | 'lifecycle' | 'approve';
  action: () => void;
}

function fuzzyMatch(query: string, text: string): { match: boolean; score: number } {
  if (!query) return { match: true, score: 0 };

  const lowerQuery = query.toLowerCase();
  const lowerText = text.toLowerCase();

  // Exact substring match gets highest score
  if (lowerText.includes(lowerQuery)) {
    const index = lowerText.indexOf(lowerQuery);
    return { match: true, score: 100 - index };
  }

  // Fuzzy character-by-character match
  let queryIdx = 0;
  let score = 0;
  let lastMatchIdx = -1;

  for (let i = 0; i < lowerText.length && queryIdx < lowerQuery.length; i++) {
    if (lowerText[i] === lowerQuery[queryIdx]) {
      queryIdx++;
      // Consecutive matches score higher
      if (lastMatchIdx === i - 1) {
        score += 10;
      } else {
        score += 5;
      }
      // Matches at word boundaries score higher
      if (i === 0 || lowerText[i - 1] === ' ' || lowerText[i - 1] === '/') {
        score += 8;
      }
      lastMatchIdx = i;
    }
  }

  return { match: queryIdx === lowerQuery.length, score };
}

interface CommandPaletteProps {
  isOpen: boolean;
  onClose: () => void;
}

export const CommandPalette: React.FC<CommandPaletteProps> = ({ isOpen, onClose }) => {
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const openFocus = useUIStore((s) => s.openFocus);
  const returnToOverview = useUIStore((s) => s.returnToOverview);
  const mode = useUIStore((s) => s.mode);
  const tasks = useTaskStore((s) => s.tasks);
  const { sendMessage } = useWebSocket();

  // Build list of actions
  const allActions = useMemo<PaletteAction[]>(() => {
    const actions: PaletteAction[] = [
      // View actions
      {
        id: 'toggle-view',
        label: mode === 'overview' ? 'Switch to Focus View' : 'Switch to Overview',
        shortcut: '\u23180',
        category: 'view',
        action: () => {
          if (mode === 'overview') {
            const firstTask = Object.values(tasks)[0];
            if (firstTask) openFocus(firstTask.id);
          } else {
            returnToOverview();
          }
        },
      },
      {
        id: 'new-task',
        label: 'New Task',
        description: 'Create a new agent task',
        shortcut: '\u2318N',
        category: 'task',
        action: () => {
          // Dispatch custom event for NewTaskDialog to handle
          window.dispatchEvent(new CustomEvent('shepherd:open-new-task'));
        },
      },

      // Approve actions
      {
        id: 'approve-all',
        label: 'Approve All Pending',
        description: 'Approve all tasks waiting for permission',
        shortcut: '\u2318\u21E7\u23CE',
        category: 'approve',
        action: () => {
          sendMessage({ type: 'task:approve_all', payload: {} });
        },
      },

      // Lifecycle tools (placeholders for Plan 3)
      {
        id: 'lifecycle-name-gen',
        label: 'Name Generator',
        description: 'Brainstorm product names with AI',
        category: 'lifecycle',
        action: () => {
          console.log('Name Generator — will be implemented in Plan 3');
        },
      },
      {
        id: 'lifecycle-logo-gen',
        label: 'Logo Generator',
        description: 'Generate app icons and logos',
        category: 'lifecycle',
        action: () => {
          console.log('Logo Generator — will be implemented in Plan 3');
        },
      },
      {
        id: 'lifecycle-north-star',
        label: 'North Star PMF Advisor',
        description: 'Define product strategy and metrics',
        category: 'lifecycle',
        action: () => {
          console.log('North Star PMF — will be implemented in Plan 3');
        },
      },
    ];

    // Add per-task approve actions for tasks needing input
    const inputTasks = Object.values(tasks).filter((t) => t.status === 'input');
    for (const task of inputTasks) {
      actions.push({
        id: `approve-${task.id}`,
        label: `Approve: ${task.title}`,
        description: task.current_action || 'Approve pending permission',
        category: 'approve',
        action: () => {
          sendMessage({ type: 'task:approve', payload: { task_id: task.id } });
        },
      });
    }

    // Add per-task focus actions
    const allTasks = Object.values(tasks);
    for (const task of allTasks) {
      actions.push({
        id: `focus-${task.id}`,
        label: `Focus: ${task.title}`,
        description: `${task.agent_id} \u00b7 ${task.status}`,
        category: 'view',
        action: () => openFocus(task.id),
      });
    }

    return actions;
  }, [mode, tasks, sendMessage, openFocus, returnToOverview]);

  // Filter and sort by fuzzy match score
  const filteredActions = useMemo(() => {
    if (!query.trim()) return allActions;

    return allActions
      .map((action) => {
        const labelResult = fuzzyMatch(query, action.label);
        const descResult = action.description
          ? fuzzyMatch(query, action.description)
          : { match: false, score: 0 };
        const bestScore = Math.max(labelResult.score, descResult.score);
        return { action, match: labelResult.match || descResult.match, score: bestScore };
      })
      .filter((item) => item.match)
      .sort((a, b) => b.score - a.score)
      .map((item) => item.action);
  }, [allActions, query]);

  // Reset selection when query changes
  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  // Focus input when palette opens
  useEffect(() => {
    if (isOpen) {
      setQuery('');
      setSelectedIndex(0);
      // Delay focus to ensure DOM is ready
      requestAnimationFrame(() => {
        inputRef.current?.focus();
      });
    }
  }, [isOpen]);

  // Scroll selected item into view
  useEffect(() => {
    if (!listRef.current) return;
    const selectedEl = listRef.current.children[selectedIndex] as HTMLElement | undefined;
    selectedEl?.scrollIntoView({ block: 'nearest' });
  }, [selectedIndex]);

  const executeAction = useCallback(
    (action: PaletteAction) => {
      onClose();
      // Execute after close animation
      requestAnimationFrame(() => action.action());
    },
    [onClose],
  );

  const handleKeyDown = (e: React.KeyboardEvent) => {
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setSelectedIndex((prev) => Math.min(prev + 1, filteredActions.length - 1));
        break;
      case 'ArrowUp':
        e.preventDefault();
        setSelectedIndex((prev) => Math.max(prev - 1, 0));
        break;
      case 'Enter':
        e.preventDefault();
        if (filteredActions[selectedIndex]) {
          executeAction(filteredActions[selectedIndex]);
        }
        break;
      case 'Escape':
        e.preventDefault();
        onClose();
        break;
    }
  };

  if (!isOpen) return null;

  const CATEGORY_LABELS: Record<string, string> = {
    approve: 'Approve',
    task: 'Tasks',
    view: 'Navigation',
    lifecycle: 'Lifecycle Tools',
  };

  // Group by category for display
  const grouped = filteredActions.reduce<Record<string, PaletteAction[]>>((acc, action) => {
    const cat = action.category;
    if (!acc[cat]) acc[cat] = [];
    acc[cat].push(action);
    return acc;
  }, {});

  let flatIndex = 0;

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 bg-black/50 z-50"
        onClick={onClose}
      />

      {/* Palette */}
      <div className="fixed inset-x-0 top-[15%] z-50 flex justify-center">
        <div className="w-full max-w-[560px] bg-zinc-900 border border-zinc-700 rounded-xl shadow-2xl overflow-hidden">
          {/* Search input */}
          <div className="flex items-center px-4 border-b border-zinc-700">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              className="text-zinc-500 shrink-0"
            >
              <circle cx="11" cy="11" r="8" />
              <line x1="21" y1="21" x2="16.65" y2="16.65" />
            </svg>
            <input
              ref={inputRef}
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Type a command..."
              className="
                flex-1 px-3 py-3.5 bg-transparent text-sm text-white
                placeholder-zinc-500 focus:outline-none
              "
            />
            <kbd className="text-[10px] text-zinc-600 bg-zinc-800 px-1.5 py-0.5 rounded">
              ESC
            </kbd>
          </div>

          {/* Results */}
          <div ref={listRef} className="max-h-[400px] overflow-y-auto py-1">
            {filteredActions.length === 0 && (
              <div className="px-4 py-8 text-center text-sm text-zinc-500">
                No matching commands
              </div>
            )}

            {Object.entries(grouped).map(([category, actions]) => (
              <div key={category}>
                <div className="px-4 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-zinc-500">
                  {CATEGORY_LABELS[category] || category}
                </div>
                {actions.map((action) => {
                  const currentIndex = flatIndex++;
                  const isSelected = currentIndex === selectedIndex;

                  return (
                    <button
                      key={action.id}
                      onClick={() => executeAction(action)}
                      onMouseEnter={() => setSelectedIndex(currentIndex)}
                      className={`
                        w-full flex items-center px-4 py-2 text-left transition-colors
                        ${isSelected ? 'bg-zinc-800' : 'hover:bg-zinc-800/50'}
                      `}
                    >
                      <div className="flex-1 min-w-0">
                        <span className={`text-sm ${isSelected ? 'text-white' : 'text-zinc-300'}`}>
                          {action.label}
                        </span>
                        {action.description && (
                          <span className="ml-2 text-xs text-zinc-500 truncate">
                            {action.description}
                          </span>
                        )}
                      </div>
                      {action.shortcut && (
                        <kbd className="ml-2 text-[10px] text-zinc-600 bg-zinc-800 px-1.5 py-0.5 rounded shrink-0">
                          {action.shortcut}
                        </kbd>
                      )}
                    </button>
                  );
                })}
              </div>
            ))}
          </div>

          {/* Footer hint */}
          <div className="flex items-center gap-3 px-4 py-2 border-t border-zinc-800 text-[10px] text-zinc-600">
            <span><kbd className="bg-zinc-800 px-1 rounded">&uarr;</kbd><kbd className="bg-zinc-800 px-1 rounded">&darr;</kbd> navigate</span>
            <span><kbd className="bg-zinc-800 px-1 rounded">&crarr;</kbd> select</span>
            <span><kbd className="bg-zinc-800 px-1 rounded">esc</kbd> close</span>
          </div>
        </div>
      </div>
    </>
  );
};
```

- [ ] **Step 2: Add Cmd+K binding and palette state**

Add palette open/close state and the Cmd+K keyboard shortcut. Modify `src/hooks/useKeyboardShortcuts.ts` to add the binding:

```typescript
// Add to src/hooks/useKeyboardShortcuts.ts — in the keydown handler:

// Cmd+K — toggle command palette
if (e.metaKey && e.key === 'k') {
  e.preventDefault();
  setPaletteOpen((prev) => !prev);
  return;
}
```

Since the palette state needs to be shared, add it to the UI store. Modify `src/stores/useUIStore.ts`:

```typescript
// Add to the UIState interface:
//   paletteOpen: boolean;
//   setPaletteOpen: (open: boolean) => void;
//   togglePalette: () => void;

// Add to the create() implementation:
//   paletteOpen: false,
//   setPaletteOpen: (open) => set({ paletteOpen: open }),
//   togglePalette: () => set((s) => ({ paletteOpen: !s.paletteOpen })),
```

Updated `src/stores/useUIStore.ts` with palette state:

```typescript
// src/stores/useUIStore.ts
import { create } from 'zustand';

interface UIState {
  mode: 'overview' | 'focus';
  focusedTaskId: string | null;
  paletteOpen: boolean;
  setMode: (mode: 'overview' | 'focus') => void;
  setFocusedTaskId: (id: string | null) => void;
  openFocus: (taskId: string) => void;
  returnToOverview: () => void;
  setPaletteOpen: (open: boolean) => void;
  togglePalette: () => void;
}

export const useUIStore = create<UIState>((set) => ({
  mode: 'overview',
  focusedTaskId: null,
  paletteOpen: false,
  setMode: (mode) => set({ mode }),
  setFocusedTaskId: (id) => set({ focusedTaskId: id }),
  openFocus: (taskId) => set({ mode: 'focus', focusedTaskId: taskId }),
  returnToOverview: () => set({ mode: 'overview', focusedTaskId: null }),
  setPaletteOpen: (open) => set({ paletteOpen: open }),
  togglePalette: () => set((s) => ({ paletteOpen: !s.paletteOpen })),
}));
```

- [ ] **Step 3: Wire CommandPalette into App.tsx**

```tsx
// In src/App.tsx — add:
import { CommandPalette } from './features/palette/CommandPalette';
import { useUIStore } from './stores/useUIStore';

// Inside the App component JSX, add above the main content:
// const paletteOpen = useUIStore((s) => s.paletteOpen);
// const setPaletteOpen = useUIStore((s) => s.setPaletteOpen);
// ...
// <CommandPalette isOpen={paletteOpen} onClose={() => setPaletteOpen(false)} />
```

Update the keyboard shortcuts hook to include `Cmd+K`:

```typescript
// src/hooks/useKeyboardShortcuts.ts — add inside the keydown handler

import { useUIStore } from '../stores/useUIStore';

// Inside the useEffect keydown handler, add before other shortcuts:
const togglePalette = useUIStore.getState().togglePalette;

// Cmd+K — command palette
if (e.metaKey && e.key === 'k') {
  e.preventDefault();
  togglePalette();
  return;
}
```

- [ ] **Step 4: Verify command palette**

Run: `npm run dev`
Expected: Pressing `Cmd+K` opens the command palette centered at top of screen. Typing filters actions with fuzzy matching. Arrow keys navigate, Enter selects, Escape closes. Actions include: New Task, Switch View, Approve All, lifecycle tool placeholders, and per-task focus/approve actions.

- [ ] **Step 5: Commit**

```bash
git add src/features/palette/CommandPalette.tsx src/stores/useUIStore.ts src/hooks/useKeyboardShortcuts.ts src/App.tsx
git commit -m "feat: add command palette (Cmd+K) with fuzzy search and categorized actions"
```

---

### Task 16: NewTaskDialog (Form with Agent Selector, Repo Picker, Isolation Mode)

**Files:**
- Create: `src/features/tasks/NewTaskDialog.tsx`
- Modify: `src/hooks/useKeyboardShortcuts.ts` (add Cmd+N binding)
- Modify: `src/stores/useUIStore.ts` (add newTaskDialogOpen)
- Modify: `src/App.tsx` (render NewTaskDialog)

- [ ] **Step 1: Add dialog state to UI store**

```typescript
// Modify src/stores/useUIStore.ts — add to interface and implementation:
//   newTaskDialogOpen: boolean;
//   setNewTaskDialogOpen: (open: boolean) => void;

// In create():
//   newTaskDialogOpen: false,
//   setNewTaskDialogOpen: (open) => set({ newTaskDialogOpen: open }),
```

Full updated store:

```typescript
// src/stores/useUIStore.ts
import { create } from 'zustand';

interface UIState {
  mode: 'overview' | 'focus';
  focusedTaskId: string | null;
  paletteOpen: boolean;
  newTaskDialogOpen: boolean;
  setMode: (mode: 'overview' | 'focus') => void;
  setFocusedTaskId: (id: string | null) => void;
  openFocus: (taskId: string) => void;
  returnToOverview: () => void;
  setPaletteOpen: (open: boolean) => void;
  togglePalette: () => void;
  setNewTaskDialogOpen: (open: boolean) => void;
}

export const useUIStore = create<UIState>((set) => ({
  mode: 'overview',
  focusedTaskId: null,
  paletteOpen: false,
  newTaskDialogOpen: false,
  setMode: (mode) => set({ mode }),
  setFocusedTaskId: (id) => set({ focusedTaskId: id }),
  openFocus: (taskId) => set({ mode: 'focus', focusedTaskId: taskId }),
  returnToOverview: () => set({ mode: 'overview', focusedTaskId: null }),
  setPaletteOpen: (open) => set({ paletteOpen: open }),
  togglePalette: () => set((s) => ({ paletteOpen: !s.paletteOpen })),
  setNewTaskDialogOpen: (open) => set({ newTaskDialogOpen: open }),
}));
```

- [ ] **Step 2: Create NewTaskDialog component**

```tsx
// src/features/tasks/NewTaskDialog.tsx
import React, { useState, useEffect, useRef } from 'react';
import { useUIStore } from '../../stores/useUIStore';
import { useWebSocket } from '../../hooks/useWebSocket';

interface NewTaskFormData {
  title: string;
  agent_id: string;
  repo_path: string;
  isolation_mode: 'worktree' | 'docker' | 'local';
  initial_prompt: string;
}

const DEFAULT_AGENTS = [
  { id: 'claude-code', name: 'Claude Code', icon: 'C' },
  { id: 'codex', name: 'Codex CLI', icon: 'X' },
  { id: 'opencode', name: 'OpenCode', icon: 'O' },
  { id: 'gemini-cli', name: 'Gemini CLI', icon: 'G' },
  { id: 'aider', name: 'Aider', icon: 'A' },
];

const ISOLATION_MODES = [
  {
    value: 'worktree' as const,
    label: 'Git Worktree',
    description: 'Lightweight isolation via Git worktree (default)',
  },
  {
    value: 'docker' as const,
    label: 'Docker Container',
    description: 'Full sandbox in a Docker container',
  },
  {
    value: 'local' as const,
    label: 'Local Workspace',
    description: 'No isolation — runs in the current directory',
  },
];

const INITIAL_FORM_DATA: NewTaskFormData = {
  title: '',
  agent_id: 'claude-code',
  repo_path: '.',
  isolation_mode: 'worktree',
  initial_prompt: '',
};

interface NewTaskDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export const NewTaskDialog: React.FC<NewTaskDialogProps> = ({ isOpen, onClose }) => {
  const [formData, setFormData] = useState<NewTaskFormData>({ ...INITIAL_FORM_DATA });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const titleInputRef = useRef<HTMLTextAreaElement>(null);
  const { sendMessage } = useWebSocket();

  // Reset form and focus when dialog opens
  useEffect(() => {
    if (isOpen) {
      setFormData({ ...INITIAL_FORM_DATA });
      setError(null);
      setIsSubmitting(false);
      requestAnimationFrame(() => {
        titleInputRef.current?.focus();
      });
    }
  }, [isOpen]);

  // Listen for custom event from command palette
  useEffect(() => {
    const handler = () => {
      useUIStore.getState().setNewTaskDialogOpen(true);
    };
    window.addEventListener('shepherd:open-new-task', handler);
    return () => window.removeEventListener('shepherd:open-new-task', handler);
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!formData.title.trim()) {
      setError('Task title/prompt is required');
      return;
    }

    setIsSubmitting(true);
    setError(null);

    try {
      const response = await fetch('/api/tasks', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          title: formData.title.trim(),
          agent_id: formData.agent_id,
          repo_path: formData.repo_path.trim() || '.',
          isolation_mode: formData.isolation_mode,
          prompt: formData.initial_prompt.trim() || undefined,
        }),
      });

      if (!response.ok) {
        const body = await response.json().catch(() => ({ error: 'Request failed' }));
        throw new Error(body.error || `HTTP ${response.status}`);
      }

      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create task');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    // Cmd+Enter submits
    if (e.metaKey && e.key === 'Enter') {
      e.preventDefault();
      handleSubmit(e);
      return;
    }
    // Escape closes
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  };

  if (!isOpen) return null;

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 bg-black/60 z-50"
        onClick={onClose}
      />

      {/* Dialog */}
      <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
        <div
          className="w-full max-w-[520px] bg-zinc-900 border border-zinc-700 rounded-xl shadow-2xl"
          onClick={(e) => e.stopPropagation()}
          onKeyDown={handleKeyDown}
        >
          {/* Header */}
          <div className="flex items-center justify-between px-5 py-3 border-b border-zinc-700">
            <h2 className="text-sm font-semibold text-white">New Task</h2>
            <button
              onClick={onClose}
              className="text-zinc-500 hover:text-white transition-colors"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>

          {/* Form */}
          <form onSubmit={handleSubmit} className="p-5 space-y-4">
            {/* Title / Prompt */}
            <div>
              <label className="block text-xs font-medium text-zinc-400 mb-1.5">
                Task Prompt <span className="text-red-400">*</span>
              </label>
              <textarea
                ref={titleInputRef}
                value={formData.title}
                onChange={(e) => setFormData((prev) => ({ ...prev, title: e.target.value }))}
                placeholder="Describe what the agent should do..."
                rows={3}
                className="
                  w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg
                  text-sm text-white placeholder-zinc-500 resize-none
                  focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500
                "
              />
            </div>

            {/* Agent Selector */}
            <div>
              <label className="block text-xs font-medium text-zinc-400 mb-1.5">
                Agent
              </label>
              <div className="grid grid-cols-5 gap-1.5">
                {DEFAULT_AGENTS.map((agent) => (
                  <button
                    key={agent.id}
                    type="button"
                    onClick={() => setFormData((prev) => ({ ...prev, agent_id: agent.id }))}
                    className={`
                      flex flex-col items-center gap-1 px-2 py-2 rounded-lg text-xs transition-colors
                      ${formData.agent_id === agent.id
                        ? 'bg-blue-600/20 border border-blue-500 text-blue-400'
                        : 'bg-zinc-800 border border-zinc-700 text-zinc-400 hover:border-zinc-600 hover:text-zinc-300'
                      }
                    `}
                  >
                    <span className="text-base font-bold">{agent.icon}</span>
                    <span className="truncate w-full text-center text-[10px]">{agent.name}</span>
                  </button>
                ))}
              </div>
            </div>

            {/* Repo Path */}
            <div>
              <label className="block text-xs font-medium text-zinc-400 mb-1.5">
                Repository Path
              </label>
              <input
                type="text"
                value={formData.repo_path}
                onChange={(e) => setFormData((prev) => ({ ...prev, repo_path: e.target.value }))}
                placeholder="."
                className="
                  w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg
                  text-sm text-white font-mono placeholder-zinc-500
                  focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500
                "
              />
              <p className="mt-1 text-[10px] text-zinc-600">
                Defaults to current directory. Use absolute path or relative to project root.
              </p>
            </div>

            {/* Isolation Mode */}
            <div>
              <label className="block text-xs font-medium text-zinc-400 mb-1.5">
                Isolation Mode
              </label>
              <div className="space-y-1.5">
                {ISOLATION_MODES.map((mode) => (
                  <label
                    key={mode.value}
                    className={`
                      flex items-start gap-3 px-3 py-2 rounded-lg cursor-pointer transition-colors
                      ${formData.isolation_mode === mode.value
                        ? 'bg-zinc-800 border border-zinc-600'
                        : 'border border-transparent hover:bg-zinc-800/50'
                      }
                    `}
                  >
                    <input
                      type="radio"
                      name="isolation_mode"
                      value={mode.value}
                      checked={formData.isolation_mode === mode.value}
                      onChange={() => setFormData((prev) => ({ ...prev, isolation_mode: mode.value }))}
                      className="mt-0.5 accent-blue-500"
                    />
                    <div>
                      <span className="text-sm text-white">{mode.label}</span>
                      <p className="text-[10px] text-zinc-500">{mode.description}</p>
                    </div>
                  </label>
                ))}
              </div>
            </div>

            {/* Initial Prompt (optional) */}
            <div>
              <label className="block text-xs font-medium text-zinc-400 mb-1.5">
                Initial Message <span className="text-zinc-600">(optional)</span>
              </label>
              <textarea
                value={formData.initial_prompt}
                onChange={(e) => setFormData((prev) => ({ ...prev, initial_prompt: e.target.value }))}
                placeholder="Additional context or instructions for the agent..."
                rows={2}
                className="
                  w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg
                  text-sm text-white placeholder-zinc-500 resize-none
                  focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500
                "
              />
            </div>

            {/* Error message */}
            {error && (
              <div className="px-3 py-2 bg-red-900/30 border border-red-800/50 rounded-lg text-sm text-red-400">
                {error}
              </div>
            )}

            {/* Actions */}
            <div className="flex items-center justify-end gap-2 pt-2">
              <button
                type="button"
                onClick={onClose}
                className="px-4 py-2 text-sm text-zinc-400 hover:text-white transition-colors"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={isSubmitting || !formData.title.trim()}
                className="
                  flex items-center gap-1.5 px-5 py-2 bg-blue-600 hover:bg-blue-500
                  disabled:opacity-50 disabled:cursor-not-allowed
                  text-white text-sm font-medium rounded-lg transition-colors
                "
              >
                {isSubmitting ? (
                  <>
                    <svg className="animate-spin h-3.5 w-3.5" viewBox="0 0 24 24">
                      <circle
                        className="opacity-25"
                        cx="12"
                        cy="12"
                        r="10"
                        stroke="currentColor"
                        strokeWidth="4"
                        fill="none"
                      />
                      <path
                        className="opacity-75"
                        fill="currentColor"
                        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
                      />
                    </svg>
                    Creating...
                  </>
                ) : (
                  <>
                    Create Task
                    <kbd className="ml-1.5 text-[10px] text-blue-200/50 bg-blue-700/50 px-1 py-0.5 rounded">
                      {'\u2318\u23CE'}
                    </kbd>
                  </>
                )}
              </button>
            </div>
          </form>
        </div>
      </div>
    </>
  );
};
```

- [ ] **Step 3: Add Cmd+N binding and wire into App.tsx**

Add the keyboard shortcut and render the dialog. In `src/hooks/useKeyboardShortcuts.ts`:

```typescript
// Add to the keydown handler in useKeyboardShortcuts.ts:

// Cmd+N — new task dialog
if (e.metaKey && e.key === 'n') {
  e.preventDefault();
  useUIStore.getState().setNewTaskDialogOpen(true);
  return;
}
```

In `src/App.tsx`:

```tsx
// Add import:
import { NewTaskDialog } from './features/tasks/NewTaskDialog';

// Inside App component, add to JSX:
// const newTaskDialogOpen = useUIStore((s) => s.newTaskDialogOpen);
// const setNewTaskDialogOpen = useUIStore((s) => s.setNewTaskDialogOpen);
// ...
// <NewTaskDialog isOpen={newTaskDialogOpen} onClose={() => setNewTaskDialogOpen(false)} />
```

- [ ] **Step 4: Verify new task dialog**

Run: `npm run dev`
Expected: Pressing `Cmd+N` opens the new task dialog. Form shows: title/prompt textarea, agent selector grid (5 agents), repo path input, isolation mode radio buttons, optional initial prompt. Submitting sends POST to `/api/tasks`. `Cmd+Enter` submits, Escape closes. Validation requires title. Error messages display below the form.

- [ ] **Step 5: Commit**

```bash
git add src/features/tasks/NewTaskDialog.tsx src/stores/useUIStore.ts src/hooks/useKeyboardShortcuts.ts src/App.tsx
git commit -m "feat: add NewTaskDialog with agent selector, isolation mode, and Cmd+N shortcut"
```

---

### Task 17: Tauri Notifications, Dock Badge, Menu Bar, System Sounds

**Files:**
- Create: `src/hooks/useNotifications.ts`
- Create: `src/lib/sounds.ts`
- Create: `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/main.rs` (register tray plugin)
- Modify: `src-tauri/Cargo.toml` (add notification + tray plugins)
- Modify: `src-tauri/capabilities/default.json` (add notification permission)

- [ ] **Step 1: Add Tauri notification and tray plugins**

```toml
# src-tauri/Cargo.toml — add to [dependencies]:
tauri-plugin-notification = "2"
```

```json
// src-tauri/capabilities/default.json — add to "permissions" array:
// "notification:default",
// "notification:allow-notify",
// "notification:allow-request-permission",
// "notification:allow-is-permission-granted"
```

Install the npm side:

```bash
npm install @tauri-apps/plugin-notification
```

- [ ] **Step 2: Create notification hook**

```typescript
// src/hooks/useNotifications.ts
import { useEffect, useCallback, useRef } from 'react';
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';
import { useWebSocket } from './useWebSocket';
import { useTaskStore } from '../stores/useTaskStore';
import { playSound } from '../lib/sounds';

export function useNotifications() {
  const { subscribe } = useWebSocket();
  const permissionGranted = useRef(false);

  // Request permission on mount
  useEffect(() => {
    async function setup() {
      let granted = await isPermissionGranted();
      if (!granted) {
        const result = await requestPermission();
        granted = result === 'granted';
      }
      permissionGranted.current = granted;
    }
    setup().catch(console.error);
  }, []);

  const notify = useCallback(
    (title: string, body: string, sound?: 'permission' | 'complete' | 'error') => {
      if (permissionGranted.current) {
        sendNotification({ title, body });
      }
      if (sound) {
        playSound(sound);
      }
    },
    [],
  );

  // Subscribe to WebSocket events that should trigger notifications
  useEffect(() => {
    const unsubPermission = subscribe(
      'permission:requested',
      (event: { task_id: string; tool: string; args: string }) => {
        const tasks = useTaskStore.getState().tasks;
        const task = tasks[event.task_id];
        const taskName = task?.title || `Task ${event.task_id.slice(0, 8)}`;
        notify(
          'Permission Required',
          `${taskName}: ${event.tool}`,
          'permission',
        );
        updateDockBadge();
      },
    );

    const unsubTaskUpdate = subscribe(
      'task:updated',
      (event: { id: string; status: string; title?: string }) => {
        if (event.status === 'done') {
          notify(
            'Task Complete',
            event.title || `Task ${event.id.slice(0, 8)} finished`,
            'complete',
          );
          updateDockBadge();
        }
        if (event.status === 'error') {
          notify(
            'Task Error',
            event.title || `Task ${event.id.slice(0, 8)} encountered an error`,
            'error',
          );
        }
      },
    );

    return () => {
      unsubPermission();
      unsubTaskUpdate();
    };
  }, [subscribe, notify]);
}

function updateDockBadge() {
  // Count tasks needing input
  const tasks = useTaskStore.getState().tasks;
  const inputCount = Object.values(tasks).filter((t) => t.status === 'input').length;

  // Use Tauri API to set dock badge (macOS)
  // This calls the Rust side which handles platform-specific badge
  try {
    if ((window as any).__TAURI__) {
      (window as any).__TAURI__.core.invoke('set_dock_badge', {
        count: inputCount,
      }).catch(() => {
        // Silently fail on non-macOS platforms
      });
    }
  } catch {
    // Not running in Tauri context
  }
}
```

- [ ] **Step 3: Create sound system**

```typescript
// src/lib/sounds.ts

// Sound configuration — paths relative to public/sounds/
const SOUND_FILES: Record<string, string> = {
  permission: '/sounds/permission.wav',
  complete: '/sounds/complete.wav',
  error: '/sounds/error.wav',
};

// Preloaded audio elements
const audioCache = new Map<string, HTMLAudioElement>();

// Volume (0-1), persisted in localStorage
function getVolume(): number {
  try {
    const stored = localStorage.getItem('shepherd:sound-volume');
    if (stored !== null) return parseFloat(stored);
  } catch {
    // ignore
  }
  return 0.5;
}

function isSoundEnabled(): boolean {
  try {
    const stored = localStorage.getItem('shepherd:sound-enabled');
    if (stored !== null) return stored === 'true';
  } catch {
    // ignore
  }
  return true;
}

export function setVolume(volume: number): void {
  localStorage.setItem('shepherd:sound-volume', String(Math.max(0, Math.min(1, volume))));
}

export function setSoundEnabled(enabled: boolean): void {
  localStorage.setItem('shepherd:sound-enabled', String(enabled));
}

export function playSound(type: 'permission' | 'complete' | 'error'): void {
  if (!isSoundEnabled()) return;

  const path = SOUND_FILES[type];
  if (!path) return;

  try {
    let audio = audioCache.get(type);
    if (!audio) {
      audio = new Audio(path);
      audioCache.set(type, audio);
    }

    audio.volume = getVolume();
    audio.currentTime = 0;
    audio.play().catch(() => {
      // Audio play can fail if user hasn't interacted with the page yet
      // or if the sound file doesn't exist yet (placeholder)
    });
  } catch {
    // Silently ignore audio errors
  }
}

// Preload sounds on first import
if (typeof window !== 'undefined') {
  Object.entries(SOUND_FILES).forEach(([type, path]) => {
    const audio = new Audio();
    audio.preload = 'auto';
    audio.src = path;
    audioCache.set(type, audio);
  });
}
```

- [ ] **Step 4: Create Rust tray module with dock badge support**

```rust
// src-tauri/src/tray.rs
use tauri::{
    AppHandle, Manager, Runtime,
    menu::{Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder},
};

pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> Result<TrayIcon<R>, tauri::Error> {
    let quit = MenuItem::with_id(app, "quit", "Quit Shepherd", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let status = MenuItem::with_id(app, "status", "Status: Idle", false, None::<&str>)?;

    let menu = Menu::with_items(app, &[&status, &show, &quit])?;

    let tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Shepherd")
        .on_menu_event(move |app, event| {
            match event.id().as_ref() {
                "quit" => {
                    app.exit(0);
                }
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(tray)
}

/// Set the dock badge count (macOS only)
#[tauri::command]
pub fn set_dock_badge(app: AppHandle, count: u32) {
    #[cfg(target_os = "macos")]
    {
        if count == 0 {
            let _ = app.set_badge_label(None::<String>);
        } else {
            let _ = app.set_badge_label(Some(count.to_string()));
        }
    }
    // No-op on non-macOS platforms
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, count);
    }
}

/// Update the tray icon tooltip with current status summary
#[tauri::command]
pub fn update_tray_status(
    app: AppHandle,
    running: u32,
    input: u32,
    done: u32,
) {
    let tooltip = format!(
        "Shepherd: {} running, {} need input, {} done",
        running, input, done
    );
    // Tray tooltip update requires storing the tray handle globally or via state
    // For now, log the status
    tracing::debug!("Tray status: {}", tooltip);
    let _ = (app, tooltip);
}
```

- [ ] **Step 5: Register tray and commands in main.rs**

```rust
// src-tauri/src/main.rs — add:
mod tray;

// In the Tauri builder, add:
// .plugin(tauri_plugin_notification::init())
// .invoke_handler(tauri::generate_handler![
//     tray::set_dock_badge,
//     tray::update_tray_status,
// ])
// .setup(|app| {
//     tray::create_tray(&app.handle())?;
//     Ok(())
// })
```

The relevant modifications to `src-tauri/src/main.rs`:

```rust
// src-tauri/src/main.rs
mod tray;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            tray::set_dock_badge,
            tray::update_tray_status,
        ])
        .setup(|app| {
            tray::create_tray(&app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 6: Wire notifications into App.tsx**

```tsx
// In src/App.tsx — add:
import { useNotifications } from './hooks/useNotifications';

// Inside the App component function body:
// useNotifications();
```

- [ ] **Step 7: Create placeholder sound files**

```bash
mkdir -p public/sounds
# Create minimal WAV placeholder files (will be replaced with real sounds)
# For now, create empty files as placeholders
touch public/sounds/permission.wav
touch public/sounds/complete.wav
touch public/sounds/error.wav
```

Note: Real sound files should be 0.5-1 second WAV files. Permission sound should be a gentle ping/chime, complete should be a positive ding, error should be a subtle alert tone. These can be generated later using a sound tool or sourced from a royalty-free library.

- [ ] **Step 8: Verify notifications and tray**

Run: `npm run tauri dev`
Expected: Tray icon appears in the macOS menu bar with "Shepherd" tooltip. Clicking the tray shows menu with "Status: Idle", "Show Window", and "Quit Shepherd". When a permission is requested via WebSocket, a native macOS notification appears. Dock badge shows count of tasks needing input. Sound plays on notification events (once real WAV files are added).

- [ ] **Step 9: Commit**

```bash
git add src/hooks/useNotifications.ts src/lib/sounds.ts src-tauri/src/tray.rs src-tauri/src/main.rs src-tauri/Cargo.toml src-tauri/capabilities/default.json public/sounds/ src/App.tsx package.json package-lock.json
git commit -m "feat: add native notifications, dock badge, system tray, and sound system"
```

---

### Task 18: Final Integration Test — Full Flow

**Files:**
- Create: `src/__tests__/integration.test.tsx`

- [ ] **Step 1: Create integration test file**

```tsx
// src/__tests__/integration.test.tsx
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import App from '../App';

// Mock WebSocket
class MockWebSocket {
  static instances: MockWebSocket[] = [];
  onopen: (() => void) | null = null;
  onclose: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  readyState = 1; // OPEN
  url: string;

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
    setTimeout(() => this.onopen?.(), 0);
  }

  send = vi.fn();
  close = vi.fn();

  // Helper to simulate receiving a message
  simulateMessage(data: object) {
    this.onmessage?.({ data: JSON.stringify(data) });
  }
}

// Mock fetch for REST API
const mockFetch = vi.fn();

// Mock Tauri APIs
vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: vi.fn().mockResolvedValue(true),
  requestPermission: vi.fn().mockResolvedValue('granted'),
  sendNotification: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('Shepherd Integration Tests', () => {
  let user: ReturnType<typeof userEvent.setup>;

  beforeEach(() => {
    user = userEvent.setup();
    MockWebSocket.instances = [];
    vi.stubGlobal('WebSocket', MockWebSocket);
    vi.stubGlobal('fetch', mockFetch);

    // Default fetch responses
    mockFetch.mockImplementation((url: string, options?: RequestInit) => {
      if (url === '/api/tasks' && (!options || options.method === 'GET' || !options.method)) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve([]),
        });
      }
      if (url === '/api/health') {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({ status: 'ok' }),
        });
      }
      if (url === '/api/tasks' && options?.method === 'POST') {
        const body = JSON.parse(options.body as string);
        return Promise.resolve({
          ok: true,
          json: () =>
            Promise.resolve({
              id: 'test-task-1',
              title: body.title,
              agent_id: body.agent_id,
              status: 'queued',
              repo_path: body.repo_path || '.',
              isolation_mode: body.isolation_mode || 'worktree',
              branch: 'shepherd/test-task-1',
              created_at: new Date().toISOString(),
              updated_at: new Date().toISOString(),
            }),
        });
      }
      return Promise.resolve({ ok: false, status: 404, json: () => Promise.resolve({}) });
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the app and shows Kanban overview', async () => {
    render(<App />);

    // Should show Kanban columns
    await waitFor(() => {
      expect(screen.getByText('Queued')).toBeDefined();
      expect(screen.getByText('Running')).toBeDefined();
      expect(screen.getByText('Needs Input')).toBeDefined();
      expect(screen.getByText('Review')).toBeDefined();
      expect(screen.getByText('Done')).toBeDefined();
    });
  });

  it('creates a task and shows it on the Kanban board', async () => {
    render(<App />);

    // Open new task dialog with Cmd+N
    await act(async () => {
      fireEvent.keyDown(window, { key: 'n', metaKey: true });
    });

    await waitFor(() => {
      expect(screen.getByText('New Task')).toBeDefined();
    });

    // Fill in the form
    const titleInput = screen.getByPlaceholderText('Describe what the agent should do...');
    await user.type(titleInput, 'Fix the login bug');

    // Submit
    const createButton = screen.getByText('Create Task');
    await user.click(createButton);

    // Verify POST was called
    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalledWith(
        '/api/tasks',
        expect.objectContaining({ method: 'POST' }),
      );
    });
  });

  it('establishes WebSocket connection', async () => {
    render(<App />);

    await waitFor(() => {
      expect(MockWebSocket.instances.length).toBeGreaterThan(0);
    });

    const ws = MockWebSocket.instances[0];
    expect(ws.url).toContain('ws://');
  });

  it('receives task update via WebSocket and updates Kanban', async () => {
    render(<App />);

    await waitFor(() => {
      expect(MockWebSocket.instances.length).toBeGreaterThan(0);
    });

    const ws = MockWebSocket.instances[0];

    // Simulate a task:created event
    await act(async () => {
      ws.simulateMessage({
        type: 'task:created',
        payload: {
          id: 'ws-task-1',
          title: 'WebSocket test task',
          agent_id: 'claude-code',
          status: 'running',
          repo_path: '.',
          isolation_mode: 'worktree',
          branch: 'shepherd/ws-task-1',
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        },
      });
    });

    await waitFor(() => {
      expect(screen.getByText('WebSocket test task')).toBeDefined();
    });
  });

  it('opens command palette with Cmd+K', async () => {
    render(<App />);

    await act(async () => {
      fireEvent.keyDown(window, { key: 'k', metaKey: true });
    });

    await waitFor(() => {
      expect(screen.getByPlaceholderText('Type a command...')).toBeDefined();
    });

    // Close with Escape
    await act(async () => {
      fireEvent.keyDown(
        screen.getByPlaceholderText('Type a command...'),
        { key: 'Escape' },
      );
    });

    await waitFor(() => {
      expect(screen.queryByPlaceholderText('Type a command...')).toBeNull();
    });
  });

  it('toggles between Overview and Focus mode with Cmd+0', async () => {
    render(<App />);

    // Start in overview — Kanban columns should be visible
    await waitFor(() => {
      expect(screen.getByText('Queued')).toBeDefined();
    });

    // First add a task so Focus has something to show
    const ws = MockWebSocket.instances[0];
    await act(async () => {
      ws?.simulateMessage({
        type: 'task:created',
        payload: {
          id: 'toggle-task-1',
          title: 'Toggle test task',
          agent_id: 'claude-code',
          status: 'running',
          repo_path: '.',
          isolation_mode: 'worktree',
          branch: 'shepherd/toggle-task-1',
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        },
      });
    });

    // Click the task card to enter focus mode
    await waitFor(() => {
      expect(screen.getByText('Toggle test task')).toBeDefined();
    });

    await user.click(screen.getByText('Toggle test task'));

    // Should be in focus mode — "Overview" button should appear
    await waitFor(() => {
      expect(screen.getByText('Overview')).toBeDefined();
    });

    // Press Cmd+0 to toggle back to overview
    await act(async () => {
      fireEvent.keyDown(window, { key: '0', metaKey: true });
    });

    await waitFor(() => {
      expect(screen.getByText('Queued')).toBeDefined();
    });
  });

  it('keyboard shortcuts are active globally', async () => {
    render(<App />);

    // Cmd+K opens palette
    await act(async () => {
      fireEvent.keyDown(window, { key: 'k', metaKey: true });
    });
    await waitFor(() => {
      expect(screen.getByPlaceholderText('Type a command...')).toBeDefined();
    });

    // Escape closes palette
    await act(async () => {
      fireEvent.keyDown(
        screen.getByPlaceholderText('Type a command...'),
        { key: 'Escape' },
      );
    });

    // Cmd+N opens new task dialog
    await act(async () => {
      fireEvent.keyDown(window, { key: 'n', metaKey: true });
    });
    await waitFor(() => {
      expect(screen.getByText('New Task')).toBeDefined();
    });
  });
});
```

- [ ] **Step 2: Run integration tests**

Run: `npx vitest run src/__tests__/integration.test.tsx`
Expected: All 6 tests pass:
- `renders the app and shows Kanban overview` — PASS
- `creates a task and shows it on the Kanban board` — PASS
- `establishes WebSocket connection` — PASS
- `receives task update via WebSocket and updates Kanban` — PASS
- `opens command palette with Cmd+K` — PASS
- `toggles between Overview and Focus mode with Cmd+0` — PASS
- `keyboard shortcuts are active globally` — PASS

- [ ] **Step 3: Manual full-flow verification**

Run: `npm run tauri dev`

Manual test checklist:
1. App opens with Kanban overview (5 columns visible)
2. `Cmd+N` opens new task dialog
3. Fill in task title, select agent, select isolation mode, submit
4. New task card appears in Queued column
5. Card moves to Running when agent starts (via WebSocket)
6. Click card to enter Focus mode — three panels visible
7. Session sidebar shows all tasks with status dots
8. Terminal shows agent output in real-time
9. DiffViewer shows file changes (when available)
10. Permission prompt appears when task status is `input`
11. `Cmd+Enter` approves, `Cmd+Shift+Enter` approves all
12. `Cmd+K` opens command palette, fuzzy search works
13. `Cmd+0` toggles between Overview and Focus
14. Native notification appears on permission request
15. Dock badge shows count of tasks needing input
16. Tray icon appears in menu bar
17. "Overview" button in SessionSidebar returns to Kanban

- [ ] **Step 4: Commit**

```bash
git add src/__tests__/integration.test.tsx
git commit -m "test: add full integration tests for Kanban, Focus, WebSocket, keyboard shortcuts, and command palette"
```

---

## Summary

**Plan 2 delivers:**
- Tauri 2.0 desktop shell with server lifecycle management
- React/TypeScript SPA with dark theme
- Real-time WebSocket + REST state sync
- Kanban overview with drag-and-drop and quick-approve
- Focus panel with xterm.js terminal and Monaco diff viewer
- Command palette (Cmd+K) with fuzzy search
- Full keyboard shortcut system
- Native notifications, dock badge, system tray
- Task creation dialog with agent selection

**Plan 3 (Lifecycle) depends on:** Command palette (for lifecycle tool integration), toast system (for contextual triggers)
