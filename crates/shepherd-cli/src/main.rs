use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "shepherd", about = "Manage your coding agents")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show status of all tasks
    Status,
    /// Create a new task
    New {
        /// Task description
        prompt: String,
        /// Agent to use
        #[arg(long, default_value = "claude-code")]
        agent: String,
        /// Isolation mode
        #[arg(long, default_value = "worktree")]
        isolation: String,
    },
    /// Approve a pending permission
    Approve {
        /// Task ID (or --all)
        task_id: Option<u64>,
        #[arg(long)]
        all: bool,
    },
    /// Initialize Shepherd in current project
    Init,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Status) => {
            println!("Shepherd status: not yet implemented");
        }
        Some(Commands::New { prompt, agent, isolation }) => {
            println!("Creating task: {prompt} (agent: {agent}, isolation: {isolation})");
        }
        Some(Commands::Approve { task_id, all }) => {
            if all {
                println!("Approving all pending permissions");
            } else if let Some(id) = task_id {
                println!("Approving task #{id}");
            }
        }
        Some(Commands::Init) => {
            println!("Initializing Shepherd...");
        }
        None => {
            println!("Starting Shepherd server + GUI...");
        }
    }
    Ok(())
}
