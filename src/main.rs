use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use std::{
    cmp::Ordering,
    fmt::Display,
    fs::{File, OpenOptions},
    io::{stdout, BufRead, BufReader, Seek, SeekFrom, Write},
    num::ParseIntError,
    path::PathBuf,
};

const DEFAULT_TASK_FILE: &str = "markdone.md";

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[clap(short, long, help = "Enable quiet mode")]
    quiet: bool,
    #[clap(short, long, help = "Specify task file")]
    file: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Add new task to task list
    Add {
        /// Task text
        task: String,
    },
    /// Mark task as complete
    Check {
        /// Task ID
        id: usize,
    },
    /// Create new task list
    Create(CreateOptions),
    /// Show tasks from task list
    List(ListOptions),
    /// Mark task as selected
    Select {
        /// Task ID
        id: usize,
    },
    /// Mark task as incomplete
    Uncheck(UncheckOptions),
    /// Deselect a selected task
    Deselect { id: usize },
}

#[derive(Debug, Parser)]
struct ListOptions {
    #[clap(short, long, help = "Show all tasks")]
    all: bool,
    #[clap(short, long, help = "Only show selected tasks")]
    selected: bool,
    #[clap(short, long, help = "Only show incomplete")]
    incomplete: bool,
    #[clap(short, long, help = "Only show complete")]
    complete: bool,
}

#[derive(Debug, Parser)]
struct UncheckOptions {
    #[clap(short, long, help = "Select task")]
    select: bool,
    /// Task ID
    id: usize,
}

#[derive(Debug, Parser)]
struct CreateOptions {
    #[clap(short, long, help = "Specify task file")]
    file: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq)]
enum TaskStatus {
    Selected,
    Incomplete,
    Complete,
}

impl Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Selected => write!(f, "selected"),
            TaskStatus::Incomplete => write!(f, "incomplete"),
            TaskStatus::Complete => write!(f, "complete"),
        }
    }
}

impl TryFrom<&String> for TaskStatus {
    type Error = anyhow::Error;
    fn try_from(section: &String) -> std::result::Result<Self, Self::Error> {
        if section == "### SELECTED" {
            return Ok(TaskStatus::Selected);
        }
        if section == "### INCOMPLETE" {
            return Ok(TaskStatus::Incomplete);
        }
        if section == "### COMPLETE" {
            return Ok(TaskStatus::Complete);
        }
        return Err(anyhow!("Error: could not find status"));
    }
}

#[derive(Debug)]
struct Task {
    id: usize,
    task: String,
    task_status: TaskStatus,
}

impl TryFrom<(String, TaskStatus)> for Task {
    type Error = anyhow::Error;
    fn try_from(
        (task, task_status): (String, TaskStatus),
    ) -> std::result::Result<Self, Self::Error> {
        let completed: bool = if task.starts_with("- [ ] **") || task.starts_with("- [x] **") {
            task.chars().nth(3).unwrap() != 'x'
        } else {
            return Err(anyhow!("Error: Start of String {:?} is not valid", task));
        };
        if completed {
            if let TaskStatus::Complete = task_status {
                return Err(anyhow!("Error: non complete task cannot be complete"));
            }
        }
        let id: usize = get_id(&task)?;
        let task: String = task
            .chars()
            .into_iter()
            .skip_while(|e| e != &':')
            .skip(2)
            .collect();
        return Ok(Task {
            id,
            task,
            task_status,
        });
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\t{}\t{}", self.task_status, self.id, self.task)
    }
}

fn get_id(task: &String) -> Result<usize, ParseIntError> {
    return task
        .chars()
        .into_iter()
        .skip(8)
        .take_while(|c| c != &'*')
        .collect::<String>()
        .parse::<usize>();
}

fn get_lines(path: &PathBuf) -> Result<Vec<String>> {
    return Ok(BufReader::new(File::open(path)?)
        .lines()
        .collect::<Result<_, _>>()?);
}

fn get_tasks_in_sections(lines: Vec<String>, sections: Vec<TaskStatus>) -> Vec<Task> {
    let mut status: Option<TaskStatus> = None;
    lines
        .into_iter()
        .filter_map(|line| {
            if let Ok(s) = TaskStatus::try_from(&line) {
                if sections.contains(&s) {
                    status = Some(s);
                }
                None
            } else if let Some(s) = status.clone() {
                match Task::try_from((line, s)) {
                    Ok(t) => Some(t),
                    Err(_) => None,
                }
            } else {
                None
            }
        })
        .collect()
}

fn get_section_start(lines: &Vec<String>, section: TaskStatus) -> Result<usize> {
    return Ok(lines
        .iter()
        .position(|value| value == &format!("### {}", section.to_string().to_uppercase()))
        .with_context(|| format!("could not find `{:?}` section start", section))?);
}

fn get_section_end(lines: &Vec<String>, section_start: usize) -> Result<usize> {
    return Ok(section_start
        + lines[section_start..lines.len()]
            .iter()
            .position(|x| x == "---")
            .with_context(|| {
                format!("could not find `{:?}` section end", lines[section_start])
            })?);
}

fn get_section_indexes(lines: &Vec<String>, section: TaskStatus) -> Result<(usize, usize)> {
    let start = get_section_start(lines, section)?;
    return Ok((start, get_section_end(lines, start)?));
}

fn get_section<'a>(lines: &'a Vec<String>, section: TaskStatus) -> Result<&'a [String]> {
    let start = get_section_start(lines, section)?;
    let end = get_section_end(lines, start)?;
    return Ok(&lines[start..end]);
}

fn get_task_count_in_section(section: &[String]) -> usize {
    return section
        .iter()
        .skip(2)
        .take_while(|e| (e.starts_with("- [ ] ") || e.starts_with("- [x] ")))
        .count();
}

fn get_task_idx_in_section(section: &[String], id: usize) -> Option<usize> {
    return section.iter().position(|e| {
        e.starts_with(&format!("- [ ] **{}**:", id))
            || e.starts_with(&String::from(format!("- [x] **{}**", id)))
    });
}

fn get_task_id(task: &String) -> Result<usize, ParseIntError> {
    return task
        .chars()
        .into_iter()
        .skip(8)
        .take_while(|e| e != &'*')
        .collect::<String>()
        .parse::<usize>();
}

fn get_next_id(lines: &Vec<String>) -> usize {
    return match lines.iter().filter_map(|e| get_task_id(e).ok()).max() {
        Some(i) => i + 1,
        None => 0,
    };
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let quiet = args.quiet;
    let path = match args.file {
        Some(p) => p,
        None => PathBuf::from(DEFAULT_TASK_FILE),
    };
    match args.command {
        Commands::Add { task } => {
            let mut lines: Vec<String> = get_lines(&path)
                .with_context(|| format!("could not read lines from file `{:?}`", path))?;
            let (section_start, section_end) = get_section_indexes(&lines, TaskStatus::Incomplete)?;

            let id = get_next_id(&lines);

            if (section_end - section_start) == 2 {
                lines.insert(section_end, String::from(""));
            }
            lines.insert(section_start + 2, format!("- [ ] **{}**: {}", id, task));
            let mut file = OpenOptions::new().write(true).open(path)?;
            file.seek(SeekFrom::Start(0))?;
            for line in lines {
                writeln!(file, "{}", line)?;
            }
            if !quiet {
                eprintln!("successfully added task `{:?}` with id `{:?}`", task, id);
            }
        }
        Commands::Check { id } => {
            let mut lines: Vec<String> = get_lines(&path)
                .with_context(|| format!("could not read lines from file `{:?}`", path))?;

            let complete_section = get_section(&lines, TaskStatus::Complete)?;

            let (task_idx, task_count): (usize, usize) =
                match get_task_idx_in_section(complete_section, id) {
                    Some(_) => {
                        bail!("task with id `{:?}` is already checked", id);
                    }
                    None => {
                        let (selected_section_start, selected_section_end) =
                            get_section_indexes(&lines, TaskStatus::Selected)?;
                        let selected_section = &lines[selected_section_start..selected_section_end];

                        match get_task_idx_in_section(selected_section, id) {
                            Some(idx) => (
                                idx + selected_section_start,
                                get_task_count_in_section(selected_section),
                            ),
                            None => {
                                let (incomplete_section_start, incomplete_section_end) =
                                    get_section_indexes(&lines, TaskStatus::Incomplete)?;
                                let incomplete_section =
                                    &lines[incomplete_section_start..incomplete_section_end];
                                match get_task_idx_in_section(incomplete_section, id) {
                                    Some(idx) => (
                                        idx + incomplete_section_start,
                                        get_task_count_in_section(incomplete_section),
                                    ),
                                    None => {
                                        bail!("could not find task with id `{:?}`", id);
                                    }
                                }
                            }
                        }
                    }
                };
            let task = lines.remove(task_idx);
            let mut chars: Vec<char> = task.chars().collect();
            chars[3] = 'x';
            let task: String = chars.into_iter().collect();
            if task_count == 1 {
                lines.remove(task_idx);
            }
            let (complete_section_start, complete_section_end) =
                get_section_indexes(&lines, TaskStatus::Complete)?;
            match (complete_section_end - complete_section_start).cmp(&2) {
                Ordering::Equal => {
                    lines.insert(complete_section_end, String::from(""));
                }
                _ => (),
            };
            lines.insert(complete_section_start + 2, task);
            let mut file = OpenOptions::new().write(true).open(path)?;
            file.set_len(lines.len() as u64)?;
            file.seek(SeekFrom::Start(0))?;

            for line in lines {
                writeln!(file, "{}", line)?;
            }
            if !quiet {
                eprintln!("successfully checked task with id `{:?}`", id);
            }
        }
        Commands::Create(options) => {
            let path = match options.file {
                Some(p) => p,
                None => path,
            };
            match path.exists() {
                true => bail!("file `{:?}` already exists", &path),
                false => {
                    let mut file = File::create(&path)
                        .with_context(|| format!("could not create file `{:?}`", &path))?;
                    file.write_all(
                        b"### SELECTED\n\n---\n\n### INCOMPLETE\n\n---\n\n### COMPLETE\n\n---\n",
                    )
                    .with_context(|| format!("could not write to file `{:?}`", &path))?;
                    if !quiet {
                        eprintln!("successfully created `{:?}`", &path);
                    }
                }
            }
        }
        Commands::List(options) => {
            let lines: Vec<String> = get_lines(&path)
                .with_context(|| format!("could not read lines from file `{:?}`", path))?;
            let mut sections: Vec<TaskStatus> = vec![];
            let list_all =
                options.all | !(options.complete | options.incomplete | options.selected);
            if options.selected | list_all {
                sections.push(TaskStatus::Selected);
            }
            if options.incomplete | list_all {
                sections.push(TaskStatus::Incomplete);
            }
            if options.complete | list_all {
                sections.push(TaskStatus::Complete);
            }
            let tasks = get_tasks_in_sections(lines, sections);
            if !quiet {
                println!("status\t\tid\ttask\n------\t\t--\t----");
            }
            let stdout = stdout();
            let mut handle = stdout.lock();
            for t in tasks.iter() {
                writeln!(handle, "{}", t)?;
            }
        }
        Commands::Select { id } => {
            let mut lines: Vec<String> = get_lines(&path)
                .with_context(|| format!("could not read lines from file `{:?}`", path))?;

            let (incomplete_section_start, incomplete_section_end) =
                get_section_indexes(&lines, TaskStatus::Incomplete)?;
            let incomplete_section = &lines[incomplete_section_start..incomplete_section_end];

            let (task_idx, task_count): (usize, usize) =
                match get_task_idx_in_section(incomplete_section, id) {
                    Some(idx) => (
                        idx + incomplete_section_start,
                        get_task_count_in_section(incomplete_section),
                    ),
                    None => {
                        let complete_section = get_section(&lines, TaskStatus::Complete)?;

                        match get_task_idx_in_section(complete_section, id) {
                            Some(_) => {
                                bail!("task with id `{:?}` is not incomplete", id);
                            }
                            None => {
                                let (selected_section_start, selected_section_end) =
                                    get_section_indexes(&lines, TaskStatus::Selected)?;
                                let selected_section =
                                    &lines[selected_section_start..selected_section_end];

                                match get_task_idx_in_section(selected_section, id) {
                                    Some(idx) => (
                                        idx + selected_section_start,
                                        get_task_count_in_section(selected_section),
                                    ),
                                    None => {
                                        bail!("could not find task with id `{:?}`", id);
                                    }
                                }
                            }
                        }
                    }
                };
            let task = lines.remove(task_idx);
            if task_count == 1 {
                lines.remove(task_idx);
            }
            let (selected_section_start, selected_section_end) =
                get_section_indexes(&lines, TaskStatus::Selected)?;

            match (selected_section_end - selected_section_start).cmp(&2) {
                Ordering::Equal => {
                    lines.insert(selected_section_end, String::from(""));
                }
                _ => (),
            };
            lines.insert(selected_section_start + 2, task);
            let mut file = OpenOptions::new().write(true).open(path)?;
            file.set_len(lines.len() as u64)?;
            file.seek(SeekFrom::Start(0))?;

            for line in lines {
                writeln!(file, "{}", line)?;
            }
            if !quiet {
                eprintln!("successfully selected task with id `{:?}`", id);
            }
        }
        Commands::Uncheck(options) => {
            let id = options.id;
            let new_section = if options.select {
                TaskStatus::Selected
            } else {
                TaskStatus::Incomplete
            };
            let mut lines: Vec<String> = get_lines(&path)
                .with_context(|| format!("could not read lines from file `{:?}`", path))?;

            let (complete_section_start, complete_section_end) =
                get_section_indexes(&lines, TaskStatus::Complete)?;
            let complete_section = &lines[complete_section_start..complete_section_end];

            let (task_idx, task_count): (usize, usize) =
                match get_task_idx_in_section(complete_section, id) {
                    Some(idx) => (
                        idx + complete_section_start,
                        get_task_count_in_section(complete_section),
                    ),
                    None => {
                        bail!("could not find task with id `{:?}` in completed tasks", id);
                    }
                };
            let task = lines.remove(task_idx);
            let mut chars: Vec<char> = task.chars().collect();
            chars[3] = ' ';
            let task: String = chars.into_iter().collect();
            if task_count == 1 {
                lines.remove(task_idx);
            }
            let (new_section_start, new_section_end) = get_section_indexes(&lines, new_section)?;
            if (new_section_end - new_section_start) == 2 {
                lines.insert(new_section_end, String::from(""));
            };
            lines.insert(new_section_start + 2, task);
            let mut file = OpenOptions::new().write(true).open(path)?;
            file.set_len(lines.len() as u64)?;
            file.seek(SeekFrom::Start(0))?;

            for line in lines {
                writeln!(file, "{}", line)?;
            }
            if !quiet {
                eprintln!("successfully unchecked task with id `{:?}`", id);
            }
        }
        Commands::Deselect { id } => {
            let mut lines: Vec<String> = get_lines(&path)
                .with_context(|| format!("could not read lines from file `{:?}`", path))?;

            let (selected_section_start, selected_section_end) =
                get_section_indexes(&lines, TaskStatus::Selected)?;
            let selected_section = &lines[selected_section_start..selected_section_end];

            let (task_idx, task_count): (usize, usize) =
                match get_task_idx_in_section(selected_section, id) {
                    Some(idx) => (
                        idx + selected_section_start,
                        get_task_count_in_section(selected_section),
                    ),
                    None => {
                        bail!("could not find task with id `{:?}`", id);
                    }
                };
            let task = lines.remove(task_idx);
            if task_count == 1 {
                lines.remove(task_idx);
            }
            let (incomplete_section_start, incomplete_section_end) =
                get_section_indexes(&lines, TaskStatus::Incomplete)?;

            match (incomplete_section_end - incomplete_section_start).cmp(&2) {
                Ordering::Equal => {
                    lines.insert(incomplete_section_end, String::from(""));
                }
                _ => (),
            };
            lines.insert(incomplete_section_start + 2, task);
            let mut file = OpenOptions::new().write(true).open(path)?;
            file.set_len(lines.len() as u64)?;
            file.seek(SeekFrom::Start(0))?;

            for line in lines {
                writeln!(file, "{}", line)?;
            }
            if !quiet {
                eprintln!("successfully deselected task with id `{:?}`", id);
            }
        }
    };
    return Ok(());
}
