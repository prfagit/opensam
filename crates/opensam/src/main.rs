//! OpenSAM - A lightweight AI agent framework

use clap::{Parser, Subcommand};
use tracing::error;

mod commands;

use commands::{
    deploy_command, engage_command, freq_status_command, init_command, schedule_add_command,
    schedule_list_command, schedule_remove_command, setup_command, status_command,
};

/// OpenSAM - AI agent for your terminal
#[derive(Parser)]
#[command(name = "sam")]
#[command(about = "◆ A lightweight AI agent framework")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize config and workspace
    Init,
    /// Chat with the agent
    Engage {
        /// Message to send
        #[arg(short, long)]
        message: Option<String>,
        /// Session ID
        #[arg(short, long, default_value = "default")]
        session: String,
    },
    /// Start gateway server
    Deploy {
        /// Verbose logging
        #[arg(short, long)]
        verbose: bool,
    },
    /// Show system status
    Status,
    /// Manage scheduled tasks
    Schedule {
        #[command(subcommand)]
        command: ScheduleCommands,
    },
    /// Manage channels
    Freq {
        #[command(subcommand)]
        command: FreqCommands,
    },
    /// Interactive setup wizard
    Setup,
}

#[derive(Subcommand)]
enum ScheduleCommands {
    /// List scheduled jobs
    List {
        #[arg(short, long)]
        all: bool,
    },
    /// Add a scheduled job
    Add {
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        message: String,
        #[arg(short, long)]
        every: Option<u64>,
        #[arg(short, long)]
        cron: Option<String>,
    },
    /// Remove a job
    Remove { id: String },
}

#[derive(Subcommand)]
enum FreqCommands {
    /// Show channel status
    Status,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize tracing based on verbose flag in Deploy command
    if matches!(cli.command, Commands::Deploy { verbose: true, .. }) {
        tracing_subscriber::fmt().with_env_filter("debug").init();
    } else {
        tracing_subscriber::fmt::init();
    }

    match cli.command {
        Commands::Init => {
            if let Err(e) = init_command().await {
                error!("Init failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Engage { message, session } => {
            if let Err(e) = engage_command(message, session).await {
                error!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Deploy { verbose: _ } => {
            if let Err(e) = deploy_command().await {
                error!("Deploy failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Status => {
            if let Err(e) = status_command().await {
                error!("Status failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Schedule { command } => match command {
            ScheduleCommands::List { all } => {
                if let Err(e) = schedule_list_command(all).await {
                    error!("Schedule list failed: {}", e);
                    std::process::exit(1);
                }
            }
            ScheduleCommands::Add {
                name,
                message,
                every,
                cron,
            } => {
                if let Err(e) = schedule_add_command(name, message, every, cron).await {
                    error!("Schedule add failed: {}", e);
                    std::process::exit(1);
                }
            }
            ScheduleCommands::Remove { id } => {
                if let Err(e) = schedule_remove_command(id).await {
                    error!("Schedule remove failed: {}", e);
                    std::process::exit(1);
                }
            }
        },
        Commands::Freq { command } => match command {
            FreqCommands::Status => {
                if let Err(e) = freq_status_command().await {
                    error!("Freq status failed: {}", e);
                    std::process::exit(1);
                }
            }
        },
        Commands::Setup => {
            if let Err(e) = setup_command().await {
                error!("Setup failed: {}", e);
                std::process::exit(1);
            }
        }
    }
}
