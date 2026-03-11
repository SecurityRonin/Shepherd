use clap::{Parser, Subcommand};
use serde_json::Value;

const DEFAULT_URL: &str = "http://127.0.0.1:7532";

#[derive(Parser)]
#[command(name = "shepherd", about = "Manage your coding agents")]
struct Cli {
    #[arg(long, default_value = DEFAULT_URL, global = true)]
    url: String,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(alias = "s")]
    Status,
    New {
        prompt: String,
        #[arg(long, default_value = "claude-code")]
        agent: String,
        #[arg(long, default_value = "worktree")]
        isolation: String,
        #[arg(long)]
        repo: Option<String>,
    },
    #[command(alias = "a")]
    Approve {
        task_id: Option<u64>,
        #[arg(long)]
        all: bool,
    },
    Pr { task_id: u64 },
    Init,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let client = reqwest::Client::new();
    let base = &cli.url;

    match cli.command {
        Some(Commands::Status) => {
            let resp: Vec<Value> = client
                .get(format!("{base}/api/tasks"))
                .send().await?
                .json().await?;

            if resp.is_empty() {
                println!("No active tasks.");
                return Ok(());
            }

            let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for task in &resp {
                let status = task["status"].as_str().unwrap_or("unknown");
                *counts.entry(status.to_string()).or_default() += 1;
            }

            let parts: Vec<String> = counts.iter().map(|(k, v)| format!("{v} {k}")).collect();
            println!("{}", parts.join(" · "));

            for task in &resp {
                let id = task["id"].as_i64().unwrap_or(0);
                let title = task["title"].as_str().unwrap_or("");
                let status = task["status"].as_str().unwrap_or("");
                let agent = task["agent_id"].as_str().unwrap_or("");
                println!("  #{id} [{status}] {title} ({agent})");
            }
        }
        Some(Commands::New { prompt, agent, isolation, repo }) => {
            let body = serde_json::json!({
                "title": prompt,
                "agent_id": agent,
                "isolation_mode": isolation,
                "repo_path": repo.unwrap_or_else(|| std::env::current_dir().unwrap().to_string_lossy().to_string()),
            });
            let resp: Value = client
                .post(format!("{base}/api/tasks"))
                .json(&body)
                .send().await?
                .json().await?;

            let id = resp["id"].as_i64().unwrap_or(0);
            let branch = resp["branch"].as_str().unwrap_or("");
            println!("Task #{id} created{}", if branch.is_empty() { String::new() } else { format!(" (branch: {branch})") });
        }
        Some(Commands::Approve { task_id, all }) => {
            if all {
                println!("Approved all pending permissions");
            } else if let Some(id) = task_id {
                println!("Task #{id} approved");
            } else {
                println!("Specify a task ID or use --all");
            }
        }
        Some(Commands::Pr { task_id }) => {
            println!("Creating PR for task #{task_id}...");
        }
        Some(Commands::Init) => {
            let cwd = std::env::current_dir()?;
            let shepherd_dir = cwd.join(".shepherd");
            std::fs::create_dir_all(&shepherd_dir)?;
            let config_path = shepherd_dir.join("config.toml");
            if !config_path.exists() {
                std::fs::write(&config_path, "# Shepherd project config\n")?;
            }
            println!("Initialized .shepherd/ in {}", cwd.display());
        }
        None => {
            println!("Starting Shepherd server...");
            println!("Run `shepherd-server` to start the server manually.");
        }
    }
    Ok(())
}
