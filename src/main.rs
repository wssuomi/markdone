use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::{fs::File, io::Write, path::PathBuf};

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
        Commands::Create => {
            let path: PathBuf = PathBuf::from("markdone.md");
            if path.exists() {
                bail!("file already exists `{:?}`", &path);
            }
            let mut file = File::create(&path)
                .with_context(|| format!("could not create file `{:?}`", &path))?;
            file.write_all(
                b"### SELECTED\n\n---\n\n### INCOMPLETE\n\n---\n\n### COMPLETE\n\n---\n",
            )
            .with_context(|| format!("could not write to file `{:?}`", &path))?;
            println!("successfully created `{:?}`", &path);
        }
        Commands::List => todo!("add list subcommand"),
        Commands::Select => todo!("add select subcommand"),
        Commands::Uncheck => todo!("add uncheck subcommand"),
    }
    return Ok(());
}
