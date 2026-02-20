use clap::{Parser, Subcommand};

const DEFAULT_TASK_DIR: &str = "tasks/";

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// List all tasks
    List,
    /// Close task
    Close { id: usize },
    /// Open task
    Open { id: usize },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    match args.command {
        Commands::List => todo!("List command"),
        Commands::Close { id } => todo!("Close command"),
        Commands::Open { id } => todo!("Open command"),
    };
    return Ok(());
}
