use clap::{Parser, Subcommand};
use std::{path::{Path, PathBuf}, process::exit};

const DEFAULT_TASK_DIR: &str = "tasks";

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

fn find_dir(target: PathBuf) -> Result<Option<PathBuf>, std::io::Error> {
    let mut p = Path::new(".");
    while p.parent().is_some() {
        let dir_content = std::fs::read_dir(p)?;
        for e in dir_content {
            if let Ok(e) = e {
                if let Ok(ef) = e.file_type() {
                    if ef.is_dir()
                        && e.file_name() == target.file_name().expect("File name should be valid")
                    {
                        return Ok(Some(e.path()));
                    }
                }
            }
        }
        p = p.parent().unwrap();
    }
    return Ok(None);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    if let Some(tasks_dir) = find_dir(PathBuf::from(DEFAULT_TASK_DIR))? {
        println!("{:?}", tasks_dir.file_name().unwrap());
    } else {
        return Err(format!("Could not find tasks directory `{}`", DEFAULT_TASK_DIR).into())
    }
    match args.command {
        Commands::List => todo!("List command"),
        Commands::Close { id } => todo!("Close command"),
        Commands::Open { id } => todo!("Open command"),
    };
    return Ok(());
}
