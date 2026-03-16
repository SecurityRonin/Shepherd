use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use reqwest::Client;
use serde_json::Value;
use std::io;

const DEFAULT_SERVER: &str = "http://localhost:7532";

#[derive(Parser)]
#[command(
    name = "shepherd",
    about = "Manage your coding agents from the command line",
    version,
    long_about = "Shepherd — a cross-platform manager for AI coding agents.\nAlias: shep"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Server URL
    #[arg(long, global = true, default_value = DEFAULT_SERVER, env = "SHEPHERD_URL")]
    server: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Show status of all tasks
    #[command(alias = "s")]
    Status,

    /// Create a new task
    New {
        /// Task description / prompt
        prompt: String,
        /// Agent to use
        #[arg(long, short, default_value = "claude-code")]
        agent: String,
        /// Isolation mode: worktree, docker, local
        #[arg(long, short, default_value = "worktree")]
        isolation: String,
        /// Repository path
        #[arg(long, short, default_value = ".")]
        repo: String,
    },

    /// Approve a pending permission
    #[command(alias = "a")]
    Approve {
        /// Task ID to approve
        task_id: Option<u64>,
        /// Approve all pending permissions
        #[arg(long)]
        all: bool,
    },

    /// Create PR for a completed task
    Pr {
        /// Task ID
        task_id: u64,
        /// Base branch
        #[arg(long, default_value = "main")]
        base: String,
    },

    /// Run quality gates for a task
    Gates {
        /// Task ID
        task_id: u64,
    },

    /// Initialize Shepherd in current project
    Init,

    /// Generate product name candidates
    #[command(alias = "name")]
    Namegen {
        /// Product description
        description: String,
        /// Vibe tags
        #[arg(long, short, num_args = 1..)]
        vibes: Vec<String>,
    },

    /// Stop all agents and server
    Stop,

    /// Generate shell completions
    Completions {
        /// Shell type
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = Client::new();
    let base_url = &cli.server;

    match cli.command {
        Some(Commands::Status) => {
            let resp = client
                .get(format!("{base_url}/api/tasks"))
                .send()
                .await?;

            if !resp.status().is_success() {
                eprintln!("Error: Could not connect to Shepherd server at {base_url}");
                eprintln!("Is the server running? Start with: shep");
                std::process::exit(1);
            }

            let tasks: Vec<Value> = resp.json().await?;

            if tasks.is_empty() {
                println!("No active tasks.");
                return Ok(());
            }

            let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for task in &tasks {
                let status = task.get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown");
                *counts.entry(status.to_string()).or_default() += 1;
            }

            let parts: Vec<String> = counts
                .iter()
                .map(|(status, count)| format!("{count} {status}"))
                .collect();
            println!("{}", parts.join(" · "));

            println!();
            for task in &tasks {
                let id = task.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                let title = task.get("title").and_then(|v| v.as_str()).unwrap_or("?");
                let status = task.get("status").and_then(|v| v.as_str()).unwrap_or("?");
                let agent = task.get("agent_id").and_then(|v| v.as_str()).unwrap_or("?");

                let status_icon = match status {
                    "queued" => "[ ]",
                    "running" => "[>]",
                    "input" => "[?]",
                    "review" => "[R]",
                    "error" => "[!]",
                    "done" => "[x]",
                    _ => "[-]",
                };

                println!("  {status_icon} #{id} {title} ({agent})");
            }
        }

        Some(Commands::New { prompt, agent, isolation, repo }) => {
            let body = serde_json::json!({
                "title": &prompt,
                "prompt": &prompt,
                "agent_id": &agent,
                "repo_path": &repo,
                "isolation_mode": &isolation,
            });

            let resp = client
                .post(format!("{base_url}/api/tasks"))
                .json(&body)
                .send()
                .await?;

            if resp.status().is_success() {
                let task: Value = resp.json().await?;
                let id = task.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                println!("Created task #{id}: {prompt}");
            } else {
                let text = resp.text().await?;
                eprintln!("Error: {text}");
                std::process::exit(1);
            }
        }

        Some(Commands::Approve { task_id, all }) => {
            if all {
                let resp = client
                    .post(format!("{base_url}/api/approve-all"))
                    .send()
                    .await?;

                if resp.status().is_success() {
                    println!("Approved all pending permissions.");
                } else {
                    eprintln!("Error: {}", resp.text().await?);
                }
            } else if let Some(id) = task_id {
                let resp = client
                    .post(format!("{base_url}/api/tasks/{id}/approve"))
                    .send()
                    .await?;

                if resp.status().is_success() {
                    println!("Approved task #{id}.");
                } else {
                    eprintln!("Error: {}", resp.text().await?);
                }
            } else {
                eprintln!("Specify a task ID or use --all");
                std::process::exit(1);
            }
        }

        Some(Commands::Pr { task_id, base }) => {
            println!("Creating PR for task #{task_id} against {base}...");

            let body = serde_json::json!({
                "base_branch": base,
                "auto_commit_message": true,
                "run_gates": true,
            });

            let resp = client
                .post(format!("{base_url}/api/tasks/{task_id}/pr"))
                .json(&body)
                .send()
                .await?;

            if resp.status().is_success() {
                let result: Value = resp.json().await?;
                if let Some(url) = result.get("pr_url").and_then(|v| v.as_str()) {
                    println!("PR created: {url}");
                } else {
                    println!("PR pipeline completed but no URL returned.");
                    if let Some(steps) = result.get("steps").and_then(|v| v.as_array()) {
                        for step in steps {
                            let name = step.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                            let status = step.get("status").and_then(|v| v.as_str()).unwrap_or("?");
                            println!("  {status}: {name}");
                        }
                    }
                }
            } else {
                eprintln!("Error: {}", resp.text().await?);
                std::process::exit(1);
            }
        }

        Some(Commands::Gates { task_id }) => {
            println!("Running quality gates for task #{task_id}...");

            let resp = client
                .post(format!("{base_url}/api/tasks/{task_id}/gates"))
                .send()
                .await?;

            if resp.status().is_success() {
                let results: Vec<Value> = resp.json().await?;
                let mut all_passed = true;

                for result in &results {
                    let name = result.get("gate_name").and_then(|v| v.as_str()).unwrap_or("?");
                    let passed = result.get("passed").and_then(|v| v.as_bool()).unwrap_or(false);
                    let ms = result.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);
                    let icon = if passed { "PASS" } else { all_passed = false; "FAIL" };
                    println!("  {icon} {name} ({ms}ms)");
                }

                if all_passed {
                    println!("\nAll gates passed.");
                } else {
                    println!("\nSome gates failed.");
                    std::process::exit(1);
                }
            } else {
                eprintln!("Error: {}", resp.text().await?);
                std::process::exit(1);
            }
        }

        Some(Commands::Init) => {
            let cwd = std::env::current_dir()?;
            let shepherd_dir = cwd.join(".shepherd");
            std::fs::create_dir_all(shepherd_dir.join("gates"))?;

            let default_config = r#"# Shepherd project configuration
# default_agent = "claude-code"
# default_isolation = "worktree"
# default_permission_mode = "ask"
"#;
            let config_path = shepherd_dir.join("config.toml");
            if !config_path.exists() {
                std::fs::write(&config_path, default_config)?;
            }

            println!("Initialized Shepherd in {}", cwd.display());
            println!("  Created .shepherd/config.toml");
            println!("  Created .shepherd/gates/");
        }

        Some(Commands::Namegen { description, vibes }) => {
            println!("Generating product names...");

            let body = serde_json::json!({
                "description": description,
                "vibes": vibes,
                "count": 20,
            });

            let resp = client
                .post(format!("{base_url}/api/namegen"))
                .json(&body)
                .send()
                .await?;

            if resp.status().is_success() {
                let result: Value = resp.json().await?;
                if let Some(candidates) = result.get("candidates").and_then(|v| v.as_array()) {
                    for (i, c) in candidates.iter().enumerate() {
                        let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                        let status = c.get("status").and_then(|v| v.as_str()).unwrap_or("?");
                        let icon = match status {
                            "all_clear" => "+",
                            "partial" => "~",
                            "conflicted" => "x",
                            _ => "?",
                        };
                        println!("  [{icon}] {:<3} {name}", i + 1);
                    }
                }
            } else {
                eprintln!("Error: {}", resp.text().await?);
                std::process::exit(1);
            }
        }

        Some(Commands::Stop) => {
            let resp = client
                .post(format!("{base_url}/api/shutdown"))
                .send()
                .await;

            match resp {
                Ok(r) if r.status().is_success() => println!("Shepherd server stopped."),
                _ => println!("Server may already be stopped."),
            }
        }

        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            generate(shell, &mut cmd, name, &mut io::stdout());
        }

        None => {
            println!("Starting Shepherd server + GUI...");
            println!("Server: {base_url}");
        }
    }

    Ok(())
}
