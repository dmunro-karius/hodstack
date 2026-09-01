mod cli;
mod front;
mod lock;
mod project;
mod skills;
mod sync;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command as ChildCommand, ExitCode};

use anyhow::{Context as _, Result, bail};
use clap::{CommandFactory, FromArgMatches};

use crate::cli::{Cli, Command};
use crate::project::Project;

pub fn run() -> Result<ExitCode> {
    let matches = Cli::command().get_matches();
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(error) => error.exit(),
    };

    let Some(command) = cli.command else {
        let Some(skill) = cli.skill else {
            Cli::command().print_help()?;
            return Ok(ExitCode::SUCCESS);
        };

        return run_skill(&skill);
    };

    match command {
        Command::Init { force } => init(force),
    }
}

pub fn report(error: &anyhow::Error) -> ExitCode {
    eprintln!("error: {error:#}");
    ExitCode::FAILURE
}

fn here() -> Result<PathBuf> {
    env::current_dir().context("cannot read the current directory")
}

fn init(force: bool) -> Result<ExitCode> {
    let root = here()?;
    let project = Project::new(&root);

    fs::create_dir_all(root.join(".hod/rules")).context("cannot create .hod/rules")?;
    fs::create_dir_all(root.join(".hod/skills")).context("cannot create .hod/skills")?;

    let intention = root.join(project::INTENTION);
    if !intention.exists() {
        fs::write(&intention, project::SEED).context("cannot write .hod/PROJECT.md")?;
        println!("  created  {}", project::INTENTION);
    }

    let mut out = std::io::stdout();
    let clean = sync::sync(&project, force, &mut out)?;

    Ok(if clean {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn find_project_root() -> Option<PathBuf> {
    let mut dir = env::current_dir().ok()?;
    loop {
        if dir.join(".hod").is_dir() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn run_skill(name: &str) -> Result<ExitCode> {
    let Some(root) = find_project_root() else {
        bail!("no `.hod` directory found here; run `hod init` first")
    };

    let project = Project::new(&root);
    let installed = project.installed_skills()?;

    if !installed.iter().any(|skill| skill.name == name) {
        bail!("no skill named `{name}`; check `.hod/skills/` or run `hod init` to see the shipped skills")
    }

    let skill_file = root
        .join(project::SKILLS_CLIENT)
        .join(name)
        .join("SKILL.md");

    if !skill_file.is_file() {
        bail!("`{name}` is not installed in `.claude/skills/`; run `hod init` first")
    }

    let status = ChildCommand::new("claude")
        .arg(format!("/{name}"))
        .current_dir(&root)
        .status()
        .context("cannot start `claude`; is Claude Code installed and on your PATH?")?;

    Ok(match status.code() {
        Some(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        None => ExitCode::FAILURE,
    })
}
