use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use std::{
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
    Add(AddOptions),
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

#[derive(Debug, Parser)]
struct AddOptions {
    /// Task text
    task: String,
    #[clap(short, long, help = "Select added task")]
    select: bool,
    #[clap(short, long, help = "Complete added task")]
    complete: bool,
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

impl TaskStatus {
    fn all() -> Vec<TaskStatus> {
        return vec![
            TaskStatus::Selected,
            TaskStatus::Incomplete,
            TaskStatus::Complete,
        ];
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

impl Task {
    fn to_markdown(&self) -> String {
        let completed = if let TaskStatus::Complete = self.task_status {
            'x'
        } else {
            ' '
        };

        return format!("- [{}] **{}**: {}", completed, self.id, self.task);
    }
}

impl TryFrom<(String, TaskStatus)> for Task {
    type Error = anyhow::Error;
    fn try_from(
        (task, task_status): (String, TaskStatus),
    ) -> std::result::Result<Self, Self::Error> {
        let completed: bool = if task.starts_with("- [ ] **") || task.starts_with("- [x] **") {
            task.chars().nth(3).unwrap() == 'x'
        } else {
            return Err(anyhow!("Error: Start of String {:?} is not valid", task));
        };
        if !completed {
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
                } else {
                    status = None;
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

fn move_task_to_section(
    id: usize,
    path: PathBuf,
    section: TaskStatus,
    allowed_sections: Vec<TaskStatus>,
) -> Result<()> {
    let lines: Vec<String> =
        get_lines(&path).with_context(|| format!("could not read lines from file `{:?}`", path))?;
    let mut tasks = get_tasks_in_sections(lines, TaskStatus::all());
    for task in tasks.iter_mut() {
        if task.id == id {
            if allowed_sections.contains(&task.task_status) {
                bail!("cannot move task from section `{:?}`", task.task_status);
            }
            task.task_status = section;
            write_tasks_to_file(path, tasks)?;
            return Ok(());
        }
    }
    bail!("could not find task with id `{:?}`", id);
}

fn write_tasks_to_file(path: PathBuf, tasks: Vec<Task>) -> Result<()> {
    let mut lines: Vec<String> = vec![];
    for (i, s) in TaskStatus::all().into_iter().enumerate() {
        lines = add_section(lines, &tasks, s);
        if i < TaskStatus::all().len() - 1 {
            lines.push(String::from(""));
        }
    }
    let mut file = OpenOptions::new().write(true).open(path)?;
    file.set_len(lines.len() as u64)?;
    file.seek(SeekFrom::Start(0))?;
    for line in lines {
        writeln!(file, "{}", line)?;
    }
    return Ok(());
}

fn add_section(mut lines: Vec<String>, tasks: &Vec<Task>, section: TaskStatus) -> Vec<String> {
    lines.push(format!("### {}", section.to_string().to_uppercase()));
    lines.push(String::from(""));
    let filtered_tasks = tasks
        .iter()
        .filter(|e| e.task_status == section)
        .collect::<Vec<&Task>>();
    if filtered_tasks.len() != 0 {
        for t in filtered_tasks {
            lines.push(t.to_markdown());
        }
        lines.push(String::from(""));
    }
    lines.push(String::from("---"));
    return lines;
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
        Commands::Add(options) => {
            let task = options.task;
            let section = if options.complete {
                TaskStatus::Complete
            } else if options.select {
                TaskStatus::Selected
            } else {
                TaskStatus::Incomplete
            };
            let completed = if section == TaskStatus::Complete {
                'x'
            } else {
                ' '
            };
            let mut lines: Vec<String> = get_lines(&path)
                .with_context(|| format!("could not read lines from file `{:?}`", path))?;
            let (section_start, section_end) = get_section_indexes(&lines, section)?;

            let id = get_next_id(&lines);

            if (section_end - section_start) == 2 {
                lines.insert(section_end, String::from(""));
            }
            lines.insert(
                section_start + 2,
                format!("- [{}] **{}**: {}", completed, id, task),
            );
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
            move_task_to_section(id, path, TaskStatus::Complete, vec![TaskStatus::Complete])?;
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
            move_task_to_section(id, path, TaskStatus::Selected, vec![])?;
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
            move_task_to_section(
                id,
                path,
                new_section,
                vec![TaskStatus::Selected, TaskStatus::Incomplete],
            )?;
            if !quiet {
                eprintln!("successfully unchecked task with id `{:?}`", id);
            }
        }
        Commands::Deselect { id } => {
            move_task_to_section(
                id,
                path,
                TaskStatus::Incomplete,
                vec![TaskStatus::Incomplete, TaskStatus::Complete],
            )?;
            if !quiet {
                eprintln!("successfully deselected task with id `{:?}`", id);
            }
        }
    };
    return Ok(());
}
