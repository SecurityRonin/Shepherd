# Competitive Analysis: Coding Agent Orchestrators & Multi-Agent Management Tools
## March 2026

---

## Executive Summary

The coding agent orchestrator space has exploded in 2025-2026. What started as terminal multiplexers for running multiple Claude Code sessions has evolved into a full category: **Agentic Development Environments (ADEs)**. The market now spans CLI tools, desktop apps, IDE integrations, and cloud platforms. Key patterns: git worktree isolation, kanban-style task boards, permission/YOLO engines, and agent-agnostic design.

**Market size signal:** Gartner predicts 33% of enterprise software will depend on agentic AI by 2028. Multi-agent system inquiries surged 1,445% from Q1 2024 to Q2 2025.

---

## Tool-by-Tool Analysis

### 1. Vibe Kanban (BloopAI)
- **Type:** Desktop app (Electron) + Web UI
- **GitHub:** ~9.4k stars, Apache 2.0
- **Stack:** Rust backend, TypeScript/React frontend
- **Key Features:**
  - Kanban board (To Do -> In Progress -> Review -> Done)
  - 10+ agent support (Claude Code, Codex, Gemini CLI, Copilot, Amp, Cursor, OpenCode, Droid, CCR, Qwen Code)
  - Isolated git worktrees per task
  - Built-in diff viewer with line-by-line review + inline comments
  - Built-in app preview (browser with devtools, inspect mode, device emulation)
  - PR creation & merge workflow
  - MCP dual-mode: client (connects to MCP servers) AND server (exposes board as API)
  - Port management daemon (dev-manager-mcp) for conflict-free dev servers
  - Task templates (standardized workflows like "bug fix")
  - Plugin system via Rust WASM (ships with Slack notifier, Jira mirror, SonarQube gate)
  - Project setup/cleanup scripts
- **UX Pattern:** Split-screen: Kanban board left, agent interaction right
- **Permission:** Runs agents with `--dangerously-skip-permissions` / `--yolo` by default (security concern flagged by users)
- **Pricing:** Free (Apache 2.0). You pay only for underlying AI API costs.
- **Strengths:**
  - Most complete kanban-based orchestrator
  - MCP server mode (board becomes an API for other agents)
  - WASM plugin system is extensible
  - Port management daemon solves real dev-server conflict pain
  - Strong code review UX with app preview
- **Weaknesses:**
  - YOLO-by-default is a security risk
  - Semantic conflicts between agents not detected (only file-level isolation)
  - No built-in quality/security gates (unlike Kintsugi)
  - No issue tracker integration (Linear, Jira, GitHub Issues)

---

### 2. Broomy
- **Type:** Desktop app (Electron, TypeScript/React)
- **Platform:** macOS only (Windows/Linux coming)
- **License:** MIT, open source
- **Key Features:**
  - Multi-session management across repos
  - Agent status detection (working, idle, finished, needs input)
  - AI-powered code review (explains what changed and why, highlights issues)
  - Built-in file explorer, git integration, terminal
  - Agent-agnostic (any terminal-based agent)
  - Notifications when agents complete tasks
- **UX Pattern:** Single window with multiple agent panels
- **Strengths:**
  - Clean "command center" UX
  - AI-assisted code review is unique differentiator
  - Agent-agnostic, easy to extend
  - MIT licensed, hackable
- **Weaknesses:**
  - macOS only currently
  - No git worktree isolation mentioned
  - No kanban view
  - No issue tracker integration
  - Smaller community, fewer stars

---

### 3. Kintsugi (Sonar)
- **Type:** Desktop app (macOS only)
- **Category:** Agentic Development Environment (ADE)
- **Key Features:**
  - Orchestrate parallel CLI agents (Claude Code, Gemini CLI, Codex)
  - Session status views (In Progress, Interrupted, Awaiting Input, Ready)
  - **Sonar-powered quality & security guardrails** (unique differentiator)
  - Plan review, request changes workflow
  - Multi-threaded development
  - Code never sent to Sonar servers (local-first)
- **UX Pattern:** Session-based dashboard with status columns
- **Permission:** Built-in guardrails for code quality/security
- **Strengths:**
  - Security/quality gates built in (SonarQube lineage)
  - Local-first, privacy-focused
  - Agent-agnostic CLI support
  - Review-first philosophy
- **Weaknesses:**
  - Experimental/prototype status
  - macOS only
  - Requires Claude Code subscription
  - Limited agent ecosystem compared to Vibe Kanban
  - No MCP integration mentioned

---

### 4. Clorch
- **Type:** CLI tool (Python, pip install)
- **Platform:** macOS (needs tmux, jq)
- **Key Features:**
  - Dashboard showing all Claude Code sessions in one place
  - **YOLO rules engine** via `~/.config/clorch/rules.yaml` (per-tool auto-approve/deny)
  - Jump to any session with one keystroke
  - Action queue for pending permissions (newest first, hotkeys)
  - Git context per agent (branch name, dirty file count)
  - **Staleness detection** (yellow >30s, red >120s idle)
  - Sound alerts (distinct macOS sounds for permission, answer, error)
  - tmux status-bar widget (agent counts at a glance)
  - Clean uninstall with `clorch uninstall`
- **UX Pattern:** Terminal dashboard with hotkey navigation
- **Permission:** Rules engine with deny/approve/YOLO per tool. First matching rule wins. Deny rules force manual review even when YOLO active.
- **Strengths:**
  - **Best rules engine** in the category (YAML-based, per-tool granularity)
  - Staleness detection is clever UX
  - Sound alerts for ambient awareness
  - tmux widget for passive monitoring
  - Lightweight, pip-installable
- **Weaknesses:**
  - Claude Code only (not agent-agnostic)
  - No git worktree isolation
  - No code review UI
  - No kanban/task management
  - macOS only (tmux dependency)

---

### 5. JetBrains Air
- **Type:** Desktop IDE (free during preview, requires JetBrains AI subscription)
- **Platform:** macOS only currently (Windows/Linux planned 2026)
- **Built on:** Fleet (discontinued Dec 2025)
- **Key Features:**
  - Supports Codex, Claude Agent, Gemini CLI, Junie out of box
  - Agent Client Protocol (ACP) support (vendor-neutral agent-editor protocol)
  - Three execution environments: Git Worktree, Docker, Local Workspace
  - Concurrent task execution with notifications
  - **Permission modes:** Ask Permission, Auto-Edit, Plan (read-only), Full Access
  - Review with inline diff comments (GitHub-style)
  - Local history snapshots for rollback
  - Context-rich task definition (mention specific line, commit, class, method)
  - Cloud execution in tech preview
- **UX Pattern:** IDE-style with agent sidebar, task switching via notifications
- **Strengths:**
  - JetBrains' 26 years of IDE expertise
  - ACP protocol support (future-proof)
  - Docker isolation option (strongest isolation)
  - Permission modes are well-designed (4 levels)
  - Context-rich task specification
- **Weaknesses:**
  - macOS only in preview
  - No issue tracker integration (Linear, Jira)
  - Inherits Fleet's RAM issues
  - Needs separate AI provider account
  - New product, still in preview

---

### 6. Claude Squad (smtg-ai)
- **Type:** CLI/TUI (Go, runs in terminal)
- **GitHub:** ~5.8k stars
- **Key Features:**
  - Manage multiple AI terminal agents (Claude Code, Aider, Codex, OpenCode, Amp)
  - tmux for session isolation
  - Git worktree isolation
  - Single TUI interface for all agents
- **UX Pattern:** Terminal-based session list with quick switching
- **Strengths:**
  - Lightweight, terminal-native
  - Multi-agent support (not just Claude)
  - Git worktree isolation
  - Good community traction
- **Weaknesses:**
  - No code review UI
  - No kanban/task management
  - No permission/rules engine
  - No issue tracker integration
  - Terminal-only (no visual diff)

---

### 7. Emdash (YC W26)
- **Type:** Desktop app (Electron/Vite)
- **Platform:** macOS, Linux, Windows
- **License:** Open source (YC W26 backed)
- **Key Features:**
  - Provider-agnostic (20+ coding agents)
  - Issue integration (Linear, Jira, GitHub, GitLab)
  - **Kanban view** of running agents
  - **Best-of-N comparison** (same task to multiple agents, compare results)
  - Remote development via SSH
  - Built-in diff & PR workflow
  - MCP support
  - Auto-detection of installed CLI agents
  - Local SQLite database (local-first)
- **UX Pattern:** Kanban board with agent cards showing status
- **Strengths:**
  - **Best-of-N is killer feature** (run same task on multiple agents/models, compare)
  - Cross-platform (mac/linux/windows)
  - Issue tracker integrations
  - Remote SSH development
  - YC-backed, active development
- **Weaknesses:**
  - Newer entrant, smaller community
  - No YOLO/permission rules engine
  - No port management
  - No WASM plugin system

---

### 8. Superinterface
- **Type:** Web platform / React library
- **Focus:** NOT a coding agent orchestrator - it's an AI assistant UI framework
- **Key Features:**
  - Build in-app AI assistants with React components
  - Multi-model support (OpenAI, Anthropic, Groq, Mistral)
  - Voice, text, custom UI modalities
  - Function calling & knowledge base
  - MCP support
- **Pricing:** From $249/month
- **Relevance to Shepherd:** Low - different category (embeddable AI UI, not agent orchestration)

---

### 9. Plandex
- **Type:** CLI/REPL (terminal-based)
- **License:** Open source
- **Key Features:**
  - 2M token context (100k per file)
  - **Diff review sandbox** (AI changes separate from project until approved)
  - Full autonomy with configurable control
  - Tree-sitter project maps (30+ languages)
  - Automated debugging (terminal + browser)
  - Multi-model support (Anthropic, OpenAI, Google, open source)
  - Cost optimization via context caching
  - Syntax checking with auto-fix (30+ languages)
  - Claude Pro/Max subscription support
- **UX Pattern:** REPL with Plan (chat) -> Tell (implement) workflow
- **Multi-Agent:** Single agent with multi-model - NOT multi-agent orchestration
- **Strengths:**
  - Excellent sandbox model (changes isolated until approved)
  - Massive context handling (2M tokens)
  - Strong syntax validation
  - Automated debugging
- **Weaknesses:**
  - Single agent only (no parallel execution)
  - Terminal-only UX
  - No kanban/visual management
  - No issue tracker integration

---

### 10. Cursor (Composer + Background Agents)
- **Type:** Desktop IDE (VS Code fork)
- **Key Features:**
  - **Composer model** (custom RL-trained, 4x faster than comparable models)
  - Multi-agent interface (multiple agents in parallel, isolated worktrees or remote)
  - **Background Agents** (isolated Ubuntu VMs with internet, open PRs)
  - Plan mode (plan with one model, build with another)
  - **Automations** (agents triggered by events: code changes, Slack, timers)
  - Subagents for parallel codebase exploration
  - Start agents from GitHub, Slack, Linear, JetBrains
  - Built-in browser tool for testing
- **Pricing:** $20/month Pro, $40/month Business, $200/month Ultra
- **Strengths:**
  - Best IDE integration (full VS Code ecosystem)
  - Custom Composer model optimized for coding
  - Automations (event-driven agent spawning)
  - Background agents with VM isolation
  - Multi-surface: IDE, Slack, GitHub, Linear, mobile
- **Weaknesses:**
  - Proprietary, closed-source
  - Vendor lock-in (VS Code fork)
  - Background agents limited in free tier
  - No self-hosting option

---

### 11. Cline (+ Cline CLI 2.0)
- **Type:** VS Code extension + CLI
- **GitHub:** Trusted by 5M+ developers
- **License:** Open source
- **Key Features:**
  - Plan/Act modes (analysis before execution)
  - Model-agnostic (any LLM provider)
  - Multi-step task execution with approval at each step
  - **CLI 2.0:** Full terminal agent with parallel sessions via tmux
  - Headless/CI-CD mode (`-y` flag, stdin/stdout piping)
  - **Agent Client Protocol (ACP)** support
  - Custom slash commands from markdown files
  - MCP tool integration
  - **Cline Teams:** SSO, RBAC, central policy, analytics
- **UX Pattern:** VS Code panel + terminal sessions
- **Strengths:**
  - Massive user base (5M+)
  - Plan/Act separation is great UX
  - ACP protocol support
  - Headless/CI mode is powerful
  - Enterprise features (Teams)
- **Weaknesses:**
  - One session per VS Code window
  - API costs can be high
  - Agent can be too aggressive with changes
  - No kanban view

---

### 12. Amp (Sourcegraph)
- **Type:** CLI + IDE extensions (VS Code, JetBrains, Neovim)
- **Key Features:**
  - Sub-agent parallelization (spawn multiple subagents)
  - Model-agnostic design
  - MCP support
  - Sourcegraph Code Graph integration (semantic codebase understanding)
  - Thread sharing & leaderboards (team features)
  - Zero Data Retention on enterprise plans
  - Unconstrained token usage
- **Pricing:** Free tier available, Team and Enterprise tiers
- **Strengths:**
  - Code graph = deep codebase understanding
  - Subagent system for parallelism
  - Team features (thread sharing, leaderboards)
  - Enterprise security (ZDR, SSO)
- **Weaknesses:**
  - Subagents are isolated (can't talk to each other)
  - Subagents less capable than main agent
  - No kanban/visual management
  - Sourcegraph integration required for full value

---

### 13. Factory.ai (Droids)
- **Type:** Cloud platform + IDE/terminal/web access
- **Key Features:**
  - Autonomous AI agents (Droids) for full SDLC
  - Ingest organizational context (GitHub, Jira, Slack, Datadog, Google Drive)
  - Multi-surface: VS Code, JetBrains, Vim, web, CLI
  - CI/CD integration (self-healing builds)
  - Sandboxed execution per Droid
  - Full audit trails
- **Funding:** $50M Series B
- **Customers:** MongoDB, EY, Zapier, Bayer
- **Claims:** 31x faster feature delivery, 96% shorter migrations
- **Pricing:** Enterprise (custom pricing)
- **Strengths:**
  - Most enterprise-ready
  - Deep organizational context ingestion
  - Self-healing CI/CD
  - Multi-surface access
- **Weaknesses:**
  - Closed source, enterprise-only pricing
  - Not developer-controllable (more autonomous)
  - High cost
  - Opaque pricing

---

### 14. Devin (Cognition)
- **Type:** Cloud-based autonomous agent (web UI)
- **Key Features:**
  - Fully autonomous software engineer
  - Sandboxed environment (shell, editor, browser)
  - Self-healing code (auto-fix failures)
  - Interactive Planning (Devin 2.0)
  - Devin Search (codebase Q&A)
  - Parallel Devin sessions
  - GitHub, Slack, Jira, Linear integration
  - Dynamic re-planning (v3.0)
- **Pricing:**
  - Core: $20/month (9 ACUs, ~2.25 hours work)
  - Team: $500/month (250 ACUs)
  - Enterprise: Custom (VPC deployment)
  - ACU = ~15 min active work
- **Enterprise:** Goldman Sachs adopted as "AI employee"
- **Strengths:**
  - Most autonomous agent available
  - Full environment (shell + editor + browser)
  - Strong enterprise traction
  - Interactive planning in 2.0
- **Weaknesses:**
  - Expensive (ACU model adds up fast)
  - Requires significant human oversight
  - Hidden cost: senior engineer time to manage
  - Cloud-only (no local/self-hosted on lower tiers)
  - Not open source

---

### 15. OpenHands (formerly OpenDevin)
- **Type:** Platform (CLI, GUI, SDK, API)
- **GitHub:** 60k+ stars, MIT license
- **Funding:** $18.8M Series A
- **Key Features:**
  - Model-agnostic (Claude, GPT, any LLM)
  - Docker/K8s sandboxed execution
  - Self-hosted or private cloud
  - Software Agent SDK (Python + REST)
  - GitHub/GitLab/Bitbucket, Slack, Jira integrations
  - Scale from 1 to thousands of agents
  - Enterprise features (governance, auditability)
- **Benchmark:** 50%+ on SWE-bench
- **Strengths:**
  - Largest open-source community (60k stars)
  - Enterprise-ready with self-hosting
  - SDK for building custom agents
  - Strong benchmark performance
- **Weaknesses:**
  - Not specifically an orchestrator (more single-agent)
  - No kanban/visual management
  - Enterprise features require paid license
  - Complex setup

---

### 16. SWE-agent (Princeton/Stanford)
- **Type:** CLI tool (Python)
- **License:** Open source (MIT)
- **Key Features:**
  - Custom Agent-Computer Interface (ACI)
  - Automated GitHub issue resolution
  - Docker isolation
  - Cybersecurity mode (EnIGMA)
  - Mini-SWE-Agent (100 lines, >74% on SWE-bench verified)
  - Multi-model support via litellm
- **Users:** Meta, NVIDIA, IBM, Nebius, Apple
- **Strengths:**
  - Academic rigor (NeurIPS 2024)
  - Excellent benchmarks
  - Mini version is remarkably simple
  - Strong research community
- **Weaknesses:**
  - Research-focused, not production-ready orchestrator
  - Single agent
  - No visual management
  - No multi-agent orchestration

---

## Additional Notable Tools

### 17. OpenAI Codex App (Feb 2026)
- **Type:** Desktop app (macOS, Windows) + CLI + Web
- **Key Features:**
  - Multi-agent parallel workflows with worktree isolation
  - Two models: gpt-5.3-codex (deep) and gpt-5.3-codex-spark (fast)
  - Interactive steering (real-time collaboration)
  - Skills & Automations (cloud-based triggers)
  - `/agent` command for thread switching
- **Pricing:** Included with ChatGPT Plus ($20/mo), Pro ($200/mo), Business ($30/user/mo)
- **Strengths:** 1M+ developers, included in existing subscriptions, strong model
- **Weaknesses:** OpenAI lock-in, not model-agnostic

### 18. GitHub Copilot Agent HQ / VS Code Multi-Agent
- **Type:** IDE integration (VS Code) + GitHub web
- **Key Features:**
  - Mission control for managing multiple agents
  - Claude + Codex + Copilot in same interface
  - Background & cloud agents
  - AGENTS.md configuration
  - Custom agents with handoffs (research -> implement -> review)
  - Fine-grained tool access controls
- **Pricing:** Copilot Pro+ or Enterprise subscription
- **Strengths:** Native to world's largest dev platform, multi-vendor
- **Weaknesses:** GitHub/VS Code lock-in, still in preview

### 19. Roo Code
- **Type:** VS Code extension (fork of Cline)
- **Key Features:**
  - Role-based modes (Architect, Code, etc.)
  - Model-agnostic (BYOK)
  - Roo Cloud for team features (Roomote, session sharing)
  - Multi-file coordinated editing
- **Strengths:** Most customizable modes, free, strong model flexibility
- **Weaknesses:** One session per VS Code window, no multi-agent orchestration

### 20. Composio Agent Orchestrator
- **Type:** CLI tool
- **Key Features:**
  - Planner + Executor dual-layer architecture
  - Dynamic tool routing (just-in-time context)
  - Stateful orchestration (structured state machine)
  - Error recovery with correction loops
  - Agent/runtime/tracker agnostic
  - Auto CI fix, auto review response
- **Strengths:** Most architecturally sophisticated orchestrator
- **Weaknesses:** Newer, less community traction

### 21. Mato (Multi-Agent Terminal Office)
- **Type:** Terminal multiplexer
- **Key Features:**
  - Office/Desk/Tab hierarchy
  - Live activity signals (spinners on active tabs)
  - Daemon-backed persistence (survives terminal close)
  - Jump mode navigation
  - Multi-session attach
  - Minimal keybinding conflicts (Rule of One: Esc only)
  - Templates (Full-Stack, Solo Dev, etc.)
  - Multilingual (EN, CN, JP, KR)
- **Strengths:** Best terminal multiplexer for agents, persistence, live signals
- **Weaknesses:** No git worktree isolation, no code review

### 22. Claude Code Native (Agent Teams, Feb 2026)
- **Type:** Built-in to Claude Code CLI
- **Key Features:**
  - Agent Teams: lead agent + teammates that talk to each other
  - Self-assign tasks from shared list
  - Challenge each other's findings
  - Demo: 16 agents built 100k-line C compiler in Rust (~$20k tokens)
- **Strengths:** Native to Claude Code, no extra tools needed
- **Weaknesses:** Claude-only, requires Claude subscription

### 23. Anthropic Claude Code Subagents/Task Tool
- **Type:** Built into Claude Code
- **Key Features:**
  - Fan-Out, Pipeline, Map-Reduce patterns
  - "Conductor" orchestration identity
  - Background agent spawning
- **Strengths:** Zero-config, built-in
- **Weaknesses:** Claude-only

---

## Comparative Matrix

| Tool | Type | Multi-Agent | Worktree | Kanban | Rules/YOLO | Code Review | Issue Tracker | MCP | Price |
|------|------|-------------|----------|--------|------------|-------------|---------------|-----|-------|
| Vibe Kanban | Desktop | 10+ agents | Yes | Yes | YOLO default | Yes (diff) | No | Client+Server | Free |
| Broomy | Desktop | Any terminal | No | No | No | AI-powered | No | No | Free |
| Kintsugi | Desktop | CLI agents | No | No | Quality gates | Yes | No | No | Free |
| Clorch | CLI | Claude only | No | No | **Best YAML rules** | No | No | No | Free |
| JetBrains Air | Desktop | 4+ agents | Yes | No | 4-level perms | Yes (inline) | No | ACP | Free preview |
| Claude Squad | CLI/TUI | 5+ agents | Yes | No | No | No | No | No | Free |
| Emdash | Desktop | 20+ agents | Yes | Yes | No | Yes (diff) | Linear/Jira/GH | Yes | Free |
| Plandex | CLI | Single | No | No | Configurable | Sandbox | No | No | Free |
| Cursor | IDE | Multi | Yes | No | Auto/manual | Yes | Linear/GH | Yes | $20-200/mo |
| Cline | VSCode+CLI | Multi (CLI 2.0) | tmux | No | Plan/Act | Approval | No | Yes | Free + API |
| Amp | CLI+IDE | Subagents | No | No | No | Git commit | No | Yes | Free/Team/Ent |
| Factory | Cloud | Droids | Sandboxed | No | Autonomous | Full audit | Jira/GH/Slack | No | Enterprise |
| Devin | Cloud | Parallel | Sandboxed | No | Interactive | Yes | GH/Slack/Jira | No | $20-500/mo |
| OpenHands | Platform | Scalable | Docker/K8s | No | No | No | GH/GL/BB | No | Free/Enterprise |
| SWE-agent | CLI | Single | Docker | No | No | No | GH issues | No | Free |
| Codex App | Desktop | Multi | Yes | No | No | Interactive | No | No | $20-200/mo |
| GitHub AHQ | IDE | Multi-vendor | Yes | No | Tool controls | Yes | GH native | No | Copilot sub |
| Composio AO | CLI | Fleet | Yes | No | No | No | GH/Linear | No | Free |
| Mato | Terminal | Multi | No | No | No | No | No | No | Free |

---

## Key UX Patterns Across Tools

### 1. Kanban Board (Visual Task Management)
- Used by: Vibe Kanban, Emdash
- Pattern: To Do -> In Progress -> Review -> Done
- Best for: Parallel task oversight, status at a glance

### 2. Terminal Dashboard
- Used by: Clorch, Claude Squad, Mato
- Pattern: Session list with status indicators, hotkey switching
- Best for: Developers who live in terminal

### 3. IDE Integration
- Used by: Cursor, Cline, Roo Code, VS Code/Copilot
- Pattern: Sidebar panels, inline diffs, chat interface
- Best for: Developers who want AI in their existing workflow

### 4. Session Grid / Desktop App
- Used by: Broomy, Kintsugi, JetBrains Air
- Pattern: Multi-pane window showing agent sessions
- Best for: Visual overview of multiple agents

### 5. Cloud Autonomous
- Used by: Devin, Factory, OpenHands
- Pattern: Web dashboard, assign tasks, review output
- Best for: Delegating entire tasks

---

## What Shepherd Can Learn / Steal

### From Vibe Kanban:
- MCP dual-mode (client AND server) -- board as API
- Port management daemon
- WASM plugin system
- Task templates

### From Clorch:
- YAML-based rules engine (per-tool approve/deny)
- Staleness detection (idle timer per agent)
- Sound alerts for ambient awareness
- tmux status-bar widget

### From Kintsugi:
- Built-in quality/security gates
- Session status categorization (In Progress, Interrupted, Awaiting Input, Ready)

### From Emdash:
- Best-of-N comparison (same task, multiple agents, compare results)
- Issue tracker integration (Linear, Jira, GitHub, GitLab)
- Remote SSH development
- Auto-detection of installed agents

### From JetBrains Air:
- 4-level permission system (Ask, Auto-Edit, Plan, Full Access)
- Docker isolation option
- Context-rich task definition (mention line/commit/class)
- ACP protocol support

### From Cursor:
- Automations (event-driven agent spawning)
- Multi-surface triggers (Slack, GitHub, Linear, mobile)
- Background agents in VMs

### From Cline:
- Plan/Act mode separation
- ACP protocol support
- Headless/CI-CD mode
- Custom slash commands from markdown

### From Composio:
- Planner + Executor dual-layer architecture
- Dynamic tool routing (just-in-time context)
- Stateful orchestration with structured state machine
- Correction loops for error recovery

### From Mato:
- Live activity signals (spinners on active tabs)
- Daemon-backed persistence
- Minimal keybinding philosophy

---

## Gaps in the Market (Shepherd Opportunities)

1. **No tool combines kanban + rules engine + quality gates**: Vibe Kanban has kanban but no rules. Clorch has rules but no kanban. Kintsugi has quality gates but limited agents.

2. **No tool has great conflict detection**: All use git worktree isolation but none detect semantic conflicts (two agents modifying related logic).

3. **No tool has good cost tracking**: Developers running 5+ agents have no visibility into per-task costs.

4. **No tool bridges terminal and GUI well**: It's either terminal-only (Clorch, Claude Squad) or GUI-only (Vibe Kanban, Emdash). No tool offers both with synchronized state.

5. **No Rust-native orchestrator exists**: All desktop apps are Electron. A native Rust UI would be faster and use less memory.

6. **No tool has intelligent agent selection**: Given a task, no tool recommends which agent/model combination would be best.

7. **No tool has persistent learning**: No orchestrator learns from past task outcomes to improve future task decomposition or agent selection.

8. **Limited webhook/automation support**: Only Cursor has event-driven agent spawning. Most tools require manual task creation.

---

## Sources

- [Vibe Kanban](https://vibekanban.com/) | [GitHub](https://github.com/BloopAI/vibe-kanban)
- [Broomy](https://broomy.org/)
- [Kintsugi](https://events.sonarsource.com/kintsugi/) | [Sonar Community](https://community.sonarsource.com/t/try-kintsugi-a-prototype-workflow-for-cli-agent-users/177606)
- [Clorch](https://github.com/androsovm/clorch)
- [JetBrains Air](https://air.dev/) | [Blog](https://blog.jetbrains.com/air/2026/03/air-launches-as-public-preview-a-new-wave-of-dev-tooling-built-on-26-years-of-experience/)
- [Claude Squad](https://github.com/smtg-ai/claude-squad)
- [Emdash](https://emdash.sh/) | [GitHub](https://github.com/generalaction/emdash)
- [Superinterface](https://superinterface.ai/)
- [Plandex](https://plandex.ai/) | [GitHub](https://github.com/plandex-ai/plandex)
- [Cursor](https://cursor.com/features) | [Cursor 2.0 Blog](https://cursor.com/blog/2-0)
- [Cline](https://cline.bot/) | [CLI 2.0](https://devops.com/cline-cli-2-0-turns-your-terminal-into-an-ai-agent-control-plane/)
- [Amp / Sourcegraph](https://ampcode.com/)
- [Factory.ai](https://factory.ai/)
- [Devin / Cognition](https://devin.ai/) | [Devin 2.0](https://venturebeat.com/programming-development/devin-2-0-is-here-cognition-slashes-price-of-ai-software-engineer-to-20-per-month-from-500/)
- [OpenHands](https://openhands.dev/) | [GitHub](https://github.com/OpenHands/OpenHands)
- [SWE-agent](https://github.com/SWE-agent/SWE-agent)
- [OpenAI Codex App](https://openai.com/codex/)
- [GitHub Agent HQ](https://github.blog/news-insights/company-news/welcome-home-agents/)
- [Roo Code](https://roocode.com/)
- [Composio Agent Orchestrator](https://github.com/ComposioHQ/agent-orchestrator)
- [Mato](https://github.com/mr-kelly/mato)
- [Awesome Agent Orchestrators](https://github.com/andyrewlee/awesome-agent-orchestrators)
- [VS Code Multi-Agent Development](https://code.visualstudio.com/blogs/2026/02/05/multi-agent-development)
