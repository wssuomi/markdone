use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Debug, Subcommand)]
enum Commands {
    Add,
    Check,
    Create,
    List,
    Select,
    Uncheck,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    match args.command {
        Commands::Add => todo!("add add subcommand"),
        Commands::Check => todo!("add check subcommand"),
        Commands::Create => todo!("add create subcommand"),
        Commands::List => todo!("add list subcommand"),
        Commands::Select => todo!("add select subcommand"),
        Commands::Uncheck => todo!("add uncheck subcommand"),
    }
    return Ok(());
}
