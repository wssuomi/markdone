use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::{
    cmp::Ordering,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    path::PathBuf,
};

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Add { task: String },
    Check,
    Create,
    List,
    Select,
    Uncheck,
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

fn get_task_count_in_section(section: &[String]) -> usize {
    return section
        .iter()
        .skip(2)
        .take_while(|e| (e.starts_with("- [ ] ") || e.starts_with("- [x] ")))
        .count();
}

fn main() -> Result<()> {
    let args = Cli::parse();
    match args.command {
        Commands::Add { task } => {
            let path: PathBuf = PathBuf::from("markdone.md");
            let mut lines: Vec<String> = get_lines(&path)
                .with_context(|| format!("could not read lines from file `{:?}`", path))?;
            let incomplete_section_start = get_section_start(&lines, "INCOMPLETE")?;
            let incomplete_section_end = get_section_end(&lines, incomplete_section_start)?;

            let complete_section_start = get_section_start(&lines, "COMPLETE")?;
            let complete_section_end = get_section_end(&lines, complete_section_start)?;

            let selected_section_start = get_section_start(&lines, "SELECTED")?;
            let selected_section_end = get_section_end(&lines, selected_section_start)?;

            let task_count: usize =
                get_task_count_in_section(&lines[incomplete_section_start..incomplete_section_end])
                    + get_task_count_in_section(
                        &lines[complete_section_start..complete_section_end],
                    )
                    + get_task_count_in_section(
                        &lines[selected_section_start..selected_section_end],
                    );

            match (incomplete_section_end - incomplete_section_start).cmp(&2) {
                Ordering::Equal => {
                    lines.insert(incomplete_section_end, String::from(""));
                }
                _ => (),
            };
            lines.insert(
                incomplete_section_start + 2,
                format!("- [ ] **{}**: {}", task_count, task),
            );
            let mut file = OpenOptions::new().write(true).open(path)?;
            file.seek(SeekFrom::Start(0))?;
            for line in lines {
                writeln!(file, "{}", line)?;
            }
            println!(
                "successfully added task `{:?}` with id `{:?}`",
                task, task_count
            );
        }
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
