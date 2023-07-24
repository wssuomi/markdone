use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Debug, Subcommand)]
enum Commands {
    List,
    Add,
    Select,
    Check,
    Uncheck,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    match args.command {
        Commands::List => todo!("add list subcommand"),
        Commands::Add => todo!("add add subcommand"),
        Commands::Select => todo!("add select subcommand"),
        Commands::Check => todo!("add check subcommand"),
        Commands::Uncheck => todo!("adduncheck subcommand"),
    }
    return Ok(());
}
