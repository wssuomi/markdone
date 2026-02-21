use clap::{Parser, Subcommand};
use std::{
    ffi::OsStr,
    fs::read_dir,
    path::{Path, PathBuf},
    process::exit,
};

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
        let dir_content = read_dir(p)?;
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

fn find_tasks(tasks_dir: PathBuf) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut tasks: Vec<PathBuf> = vec![];
    for tasks_dir_entry in read_dir(tasks_dir)? {
        let tasks_dir_entry = match tasks_dir_entry {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Skipping entry - {}", e);
                continue;
            }
        };
        if let Ok(ft) = tasks_dir_entry.file_type() {
            if !ft.is_dir() {
                continue;
            }
        }
        let dir_content = match read_dir(tasks_dir_entry.path()) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Skipping entry - {}", e);
                continue;
            }
        };
        for dir_entry in dir_content {
            let dir_entry = match dir_entry {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Skipping entry - {}", e);
                    continue;
                }
            };
            match dir_entry.file_type() {
                Ok(v) => {
                    if v.is_file() && dir_entry.file_name() == OsStr::new("TASK.md") {
                        tasks.push(dir_entry.path());
                    }
                }
                Err(e) => {
                    eprintln!("Skipping entry - {}", e);
                    continue;
                }
            }
        }
    }
    Ok(tasks)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    match args.command {
        Commands::List => {
            if let Some(tasks_dir) = find_dir(PathBuf::from(DEFAULT_TASK_DIR))? {
                if let Ok(tasks) = find_tasks(tasks_dir) {
                    println!("{:?}", tasks);
                } else {
                    return Err("Unable to get tasks".into());
                }
            } else {
                return Err(
                    format!("Could not find tasks directory `{}`", DEFAULT_TASK_DIR).into(),
                );
            }
        }
        Commands::Close { id } => todo!("Close command"),
        Commands::Open { id } => todo!("Open command"),
    };
    return Ok(());
}
