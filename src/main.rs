use anyhow::{anyhow, bail, Context, Error, Result};
use clap::{Parser, Subcommand};
use std::{
    cmp::Ordering,
    fs::{File, OpenOptions},
    io::{stdout, BufRead, BufReader, Seek, SeekFrom, Write},
    path::PathBuf,
};

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[clap(short, long, help = "Enable quiet mode")]
    quiet: bool,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Add new task to task list
    Add { task: String },
    /// Mark task as complete
    Check { id: usize },
    /// Create new task list
    Create,
    /// Show tasks from task list
    List(ListCommand),
    /// Mark task as selected
    Select { id: usize },
    /// Mark task as incomplete
    Uncheck { id: usize },
}

#[derive(Debug, Parser)]
struct ListCommand {
    #[command(subcommand)]
    command: ListCommands,
}

#[derive(Debug, Subcommand)]
enum ListCommands {
    /// Show all tasks
    All,
    /// Only show selected tasks
    SELECTED,
    /// Only show incomplete tasks
    INCOMPLETE,
    /// Only show complete tasks
    COMPLETE,
}

fn get_lines(path: &PathBuf) -> Result<Vec<String>> {
    return Ok(BufReader::new(File::open(path)?)
        .lines()
        .collect::<Result<_, _>>()?);
}

fn get_section_start(lines: &Vec<String>, section: &str) -> Result<usize> {
    let section_start: usize = lines
        .iter()
        .position(|value| value == &format!("### {}", section))
        .with_context(|| format!("could not find `{:?}` section start", section))?;
    return Ok(section_start);
}

fn get_section_end(lines: &Vec<String>, section_start: usize) -> Result<usize> {
    let section_end = section_start
        + lines[section_start..lines.len()]
            .iter()
            .position(|x| x == "---")
            .with_context(|| format!("could not find `{:?}` section end", lines[section_start]))?;
    return Ok(section_end);
}

fn get_section_indexes(lines: &Vec<String>, section: &str) -> Result<(usize, usize)> {
    let start = get_section_start(lines, section)?;
    let end = get_section_end(lines, start)?;
    return Ok((start, end));
}

fn get_section<'a>(lines: &'a Vec<String>, section: &str) -> Result<&'a [String]> {
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

fn get_tasks_in_section(section: &[String]) -> Vec<String> {
    return section
        .iter()
        .skip(2)
        .take_while(|e| (e.starts_with("- [ ] ") || e.starts_with("- [x] ")))
        .cloned()
        .collect::<Vec<String>>();
}

fn print_tasks(tasks: Vec<String>, section: &str) -> Result<()> {
    let stdout = stdout();
    let mut handle = stdout.lock();
    for t in tasks.iter() {
        let id: String = t.chars().skip(8).take_while(|x| x != &'*').collect();
        let task: String = t.chars().skip_while(|x| x != &':').skip(2).collect();
        writeln!(handle, "{}\t{}\t{}", section, id, task)?;
    }
    Ok(())
}

fn get_task_id(task: &String) -> usize {
    return task
        .chars()
        .into_iter()
        .skip(8)
        .take_while(|e| e != &'*')
        .collect::<String>()
        .parse()
        .unwrap();
}

fn get_next_id(lines: &Vec<String>) -> Result<usize> {
    let selected_tasks = get_tasks_in_section(get_section(&lines, "SELECTED")?);
    let incomplete_tasks = get_tasks_in_section(get_section(&lines, "INCOMPLETE")?);
    let complete_tasks = get_tasks_in_section(get_section(&lines, "COMPLETE")?);

    let all_tasks: Vec<String> = [selected_tasks, incomplete_tasks, complete_tasks].concat();
    if all_tasks.len() == 0 {
        return Ok(0);
    }
    let mut highest_id = 0;

    for task in all_tasks.iter() {
        let task_id = get_task_id(task);
        if task_id > highest_id {
            highest_id = task_id
        }
    }

    return Ok(highest_id + 1);
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let quiet = args.quiet;
    let result: Result<(), Error> = match args.command {
        Commands::Add { task } => {
            let path: PathBuf = PathBuf::from("markdone.md");
            let mut lines: Vec<String> = get_lines(&path)
                .with_context(|| format!("could not read lines from file `{:?}`", path))?;
            let (incomplete_section_start, incomplete_section_end) =
                get_section_indexes(&lines, "INCOMPLETE")?;

            let id = get_next_id(&lines)?;

            if let Ordering::Equal = (incomplete_section_end - incomplete_section_start).cmp(&2) {
                lines.insert(incomplete_section_end, String::from(""));
            };
            lines.insert(
                incomplete_section_start + 2,
                format!("- [ ] **{}**: {}", id, task),
            );
            let mut file = OpenOptions::new().write(true).open(path)?;
            file.seek(SeekFrom::Start(0))?;
            for line in lines {
                writeln!(file, "{}", line)?;
            }
            if !quiet {
                eprintln!("successfully added task `{:?}` with id `{:?}`", task, id);
            }
            Ok(())
        }
        Commands::Check { id } => {
            let path: PathBuf = PathBuf::from("markdone.md");
            let mut lines: Vec<String> = get_lines(&path)
                .with_context(|| format!("could not read lines from file `{:?}`", path))?;

            let complete_section = get_section(&lines, "COMPLETE")?;

            let (task_idx, task_count): (usize, usize) =
                match get_task_idx_in_section(complete_section, id) {
                    Some(_) => {
                        bail!("task with id `{:?}` is already checked", id);
                    }
                    None => {
                        let (selected_section_start, selected_section_end) =
                            get_section_indexes(&lines, "SELECTED")?;
                        let selected_section = &lines[selected_section_start..selected_section_end];

                        match get_task_idx_in_section(selected_section, id) {
                            Some(idx) => (
                                idx + selected_section_start,
                                get_task_count_in_section(selected_section),
                            ),
                            None => {
                                let (incomplete_section_start, incomplete_section_end) =
                                    get_section_indexes(&lines, "INCOMPLETE")?;
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
                get_section_indexes(&lines, "COMPLETE")?;
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
            Ok(())
        }
        Commands::Create => {
            let path: PathBuf = PathBuf::from("markdone.md");
            match path.exists() {
                true => Err(anyhow!("file already exists `{:?}`", &path)),
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
                    Ok(())
                }
            }
        }
        Commands::List(command) => match command.command {
            ListCommands::All => {
                let path: PathBuf = PathBuf::from("markdone.md");
                let lines: Vec<String> = get_lines(&path)
                    .with_context(|| format!("could not read lines from file `{:?}`", path))?;

                let selected_tasks = get_tasks_in_section(get_section(&lines, "SELECTED")?);
                let incomplete_tasks = get_tasks_in_section(get_section(&lines, "INCOMPLETE")?);
                let complete_tasks = get_tasks_in_section(get_section(&lines, "COMPLETE")?);

                if !quiet
                    && selected_tasks.len() == 0
                    && incomplete_tasks.len() == 0
                    && complete_tasks.len() == 0
                {
                    eprintln!("no tasks");
                } else {
                    if !quiet {
                        eprintln!("status\t\tid\ttask");
                        eprintln!("----------------------------");
                    }
                    print_tasks(selected_tasks, "selected")?;
                    print_tasks(incomplete_tasks, "incomplete")?;
                    print_tasks(complete_tasks, "complete")?;
                }
                Ok(())
            }
            ListCommands::SELECTED => {
                let path: PathBuf = PathBuf::from("markdone.md");
                let lines: Vec<String> = get_lines(&path)
                    .with_context(|| format!("could not read lines from file `{:?}`", path))?;
                let selected_tasks = get_tasks_in_section(get_section(&lines, "SELECTED")?);
                if !quiet && selected_tasks.len() == 0 {
                    eprintln!("no tasks");
                } else {
                    if !quiet {
                        eprintln!("status\t\tid\ttask");
                        eprintln!("----------------------------");
                    }
                    print_tasks(selected_tasks, "selected")?;
                }
                Ok(())
            }
            ListCommands::INCOMPLETE => {
                let path: PathBuf = PathBuf::from("markdone.md");
                let lines: Vec<String> = get_lines(&path)
                    .with_context(|| format!("could not read lines from file `{:?}`", path))?;
                let incomplete_tasks = get_tasks_in_section(get_section(&lines, "INCOMPLETE")?);
                if !quiet && incomplete_tasks.len() == 0 {
                    eprintln!("no tasks");
                } else {
                    if !quiet {
                        eprintln!("status\t\tid\ttask");
                        eprintln!("----------------------------");
                    }
                    print_tasks(incomplete_tasks, "incomplete")?;
                }
                Ok(())
            }
            ListCommands::COMPLETE => {
                let path: PathBuf = PathBuf::from("markdone.md");

                let lines: Vec<String> = get_lines(&path)
                    .with_context(|| format!("could not read lines from file `{:?}`", path))?;
                let complete_tasks = get_tasks_in_section(get_section(&lines, "COMPLETE")?);

                if !quiet && complete_tasks.len() == 0 {
                    eprintln!("no tasks");
                } else {
                    if !quiet {
                        eprintln!("status\t\tid\ttask");
                        eprintln!("----------------------------");
                    }
                    print_tasks(complete_tasks, "complete")?;
                }
                Ok(())
            }
        },
        Commands::Select { id } => {
            let path: PathBuf = PathBuf::from("markdone.md");
            let mut lines: Vec<String> = get_lines(&path)
                .with_context(|| format!("could not read lines from file `{:?}`", path))?;

            let (incomplete_section_start, incomplete_section_end) =
                get_section_indexes(&lines, "INCOMPLETE")?;
            let incomplete_section = &lines[incomplete_section_start..incomplete_section_end];

            let (task_idx, task_count): (usize, usize) =
                match get_task_idx_in_section(incomplete_section, id) {
                    Some(idx) => (
                        idx + incomplete_section_start,
                        get_task_count_in_section(incomplete_section),
                    ),
                    None => {
                        let complete_section = get_section(&lines, "COMPLETE")?;

                        match get_task_idx_in_section(complete_section, id) {
                            Some(_) => {
                                bail!("task with id `{:?}` is not incomplete", id);
                            }
                            None => {
                                let (selected_section_start, selected_section_end) =
                                    get_section_indexes(&lines, "SELECTED")?;
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
                get_section_indexes(&lines, "SELECTED")?;

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
            Ok(())
        }
        Commands::Uncheck { id } => {
            let path: PathBuf = PathBuf::from("markdone.md");
            let mut lines: Vec<String> = get_lines(&path)
                .with_context(|| format!("could not read lines from file `{:?}`", path))?;

            let (complete_section_start, complete_section_end) =
                get_section_indexes(&lines, "COMPLETE")?;
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
            let (incomplete_section_start, incomplete_section_end) =
                get_section_indexes(&lines, "INCOMPLETE")?;
            if let Ordering::Equal = (incomplete_section_end - incomplete_section_start).cmp(&2) {
                lines.insert(incomplete_section_end, String::from(""));
            };
            lines.insert(incomplete_section_start + 2, task);
            let mut file = OpenOptions::new().write(true).open(path)?;
            file.set_len(lines.len() as u64)?;
            file.seek(SeekFrom::Start(0))?;

            for line in lines {
                writeln!(file, "{}", line)?;
            }
            if !quiet {
                eprintln!("successfully unchecked task with id `{:?}`", id);
            }
            Ok(())
        }
    };
    match result {
        Ok(_) => std::process::exit(exitcode::OK),
        Err(e) => {
            if !quiet {
                eprintln!("Error: {}", e);
            }
            std::process::exit(exitcode::USAGE);
        }
    }
}
