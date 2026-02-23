use clap::{Parser, Subcommand};
use std::{
    env,
    ffi::OsStr,
    fmt::Display,
    fs::read_dir,
    path::{Path, PathBuf},
};

const DEFAULT_TASK_DIR: &str = "tasks";

#[derive(Debug)]
struct Task {
    path: PathBuf,
    title: String,
    status: TaskStatus,
    priority: u32,
    description: String,
}

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug)]
enum TaskStatus {
    Open,
    Closed,
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

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[ {} ]: [{}][{}] - {}",
            self.path.display(),
            self.status,
            self.priority,
            self.title
        )
    }
}

impl Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TaskStatus::Open => "Open",
                TaskStatus::Closed => "Closed",
            }
        )
    }
}

fn find_dir(target: &str) -> Result<Option<PathBuf>, std::io::Error> {
    let mut p = env::current_dir()?;
    loop {
        let dir_content = read_dir(&p)?;
        for e in dir_content {
            let e = match e {
                Ok(v) => v,
                Err(_) => continue,
            };
            if let Ok(ft) = e.file_type() {
                if !ft.is_file() && e.file_name() == OsStr::new(target) {
                    return Ok(Some(e.path()));
                }
            }
        }
        p = match p.parent() {
            Some(v) => v.to_path_buf(),
            None => return Ok(None),
        };
    }
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

fn parse_task_file(path: &Path) -> Result<Task, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let mut lines = content.lines();
    let title_line = lines.next().ok_or("Missing title line")?;
    let title = title_line
        .strip_prefix("# ")
        .ok_or(format!(
            "Expected line starting with '# ' found '{}'",
            title_line
        ))?
        .to_string();
    let empty_line = lines.next().ok_or("Missing empty line after title")?;
    if !empty_line.is_empty() {
        return Err(format!("expected empty line after title found '{}'", empty_line).into());
    }
    let status_line = lines.next().ok_or("Missing status line")?;
    let status = status_line.strip_prefix("STATUS: ").ok_or_else(|| {
        format!(
            "Expected line starting with 'STATUS: ' found '{}'",
            status_line
        )
    })?;
    let status = match status {
        "CLOSED" => TaskStatus::Closed,
        "OPEN" => TaskStatus::Open,
        _ => return Err(format!("Expected 'OPEN' or 'CLOSED' found '{status}'").into()),
    };
    let priority_line = lines.next().ok_or("Missing priority line")?;
    let priority = priority_line.strip_prefix("PRIORITY: ").ok_or_else(|| {
        format!(
            "Expected line starting with 'PRIORITY: ' found '{}'",
            priority_line
        )
    })?;
    let priority = priority
        .parse::<u32>()
        .map_err(|_| format!("invalid priority '{}'", priority))?;
    let empty_line = lines.next().ok_or("Missing empty line after fields")?;
    if !empty_line.is_empty() {
        return Err(format!("expected empty line after fields found '{}'", empty_line).into());
    }
    let description = lines.collect::<Vec<_>>().join("\n");
    return Ok(Task {
        path: PathBuf::from(path),
        title,
        status,
        priority,
        description,
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    match args.command {
        Commands::List => {
            if let Some(tasks_dir) = find_dir(DEFAULT_TASK_DIR)? {
                if let Ok(tasks) = find_tasks(tasks_dir) {
                    let mut parsed_tasks: Vec<Task> = vec![];
                    for tp in tasks {
                        match parse_task_file(tp.as_path()) {
                            Ok(t) => {
                                parsed_tasks.push(t);
                            }
                            Err(e) => {
                                eprintln!(
                                    "Warning: Unable to read task '{}' - '{}'",
                                    tp.display(),
                                    e.to_string()
                                );
                            }
                        }
                    }
                    for t in parsed_tasks {
                        println!("{}", t);
                    }
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
