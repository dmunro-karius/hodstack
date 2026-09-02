mod agent;
mod cli;
mod front;
mod help;
mod list;
mod lock;
mod project;
mod skills;
mod sync;
mod update;

use std::env;
use std::fs;
use std::io::{self, Write as _};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::OnceLock;

use anyhow::{bail, Context as _, Result};
use clap::{CommandFactory, FromArgMatches};

use crate::cli::{Cli, Command};
use crate::project::Project;
use crate::sync::Mode;

pub fn command() -> clap::Command {
    Cli::command()
        .version(version())
        .help_template(help::template())
}

pub fn version() -> &'static str {
    static VERSION: OnceLock<String> = OnceLock::new();

    VERSION
        .get_or_init(|| {
            let version = env!("CARGO_PKG_VERSION");
            match option_env!("HOD_COMMIT") {
                Some(commit) => format!("{version} ({})", commit.get(..7).unwrap_or(commit)),
                None => version.to_owned(),
            }
        })
        .as_str()
}

pub fn run() -> Result<ExitCode> {
    let matches = command().get_matches();
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(error) => error.exit(),
    };

    let Some(cmd) = cli.command else {
        let Some(skill) = cli.skill else {
            command().print_help()?;
            return Ok(ExitCode::SUCCESS);
        };

        let result = run_skill(&skill);
        update::notice(&mut io::stderr());
        return result;
    };

    match cmd {
        Command::Init { force } => {
            let result = init(force);
            update::notice(&mut io::stderr());
            result
        }
        Command::List => {
            let mut out = io::stdout();
            let result = list::list(&Project::new(&here()?), &mut out);
            update::notice(&mut io::stderr());
            result
        }
        Command::Update {
            check,
            project,
            force,
        } => {
            let mut out = io::stdout();
            refresh(check, project, force, &mut out)
        }
        Command::Completions { shell } => Ok(completions(shell)),
    }
}

pub fn report(error: &anyhow::Error) -> ExitCode {
    let _ = writeln!(io::stderr(), "error: {error:#}");
    ExitCode::FAILURE
}

fn completions(shell: clap_complete::Shell) -> ExitCode {
    let mut command = command();
    let name = command.get_name().to_owned();
    clap_complete::generate(shell, &mut command, name, &mut io::stdout());
    ExitCode::SUCCESS
}

fn here() -> Result<PathBuf> {
    env::current_dir().context("cannot read the current directory")
}

fn init(force: bool) -> Result<ExitCode> {
    let root = here()?;
    let project = Project::new(&root);
    let mut out = io::stdout();

    if !force {
        for name in [project::AGENTS, project::CLAUDE] {
            let path = project.path(name);
            let exists = path
                .try_exists()
                .with_context(|| format!("cannot read `{}`", path.display()))?;

            if exists {
                writeln!(out, "  {name} already exists. Nothing was written.")?;
                writeln!(
                    out,
                    "  Move its text to `{}`, then run `hod init` again.",
                    project::INTENTION
                )?;
                return Ok(ExitCode::FAILURE);
            }
        }
    }

    fs::create_dir_all(root.join(".hod/rules")).context("cannot create .hod/rules")?;
    fs::create_dir_all(root.join(".hod/skills")).context("cannot create .hod/skills")?;

    let intention = root.join(project::INTENTION);
    if !intention.exists() {
        fs::write(&intention, project::SEED).context("cannot write .hod/PROJECT.md")?;
        writeln!(out, "  created  {}", project::INTENTION)?;
    }

    let mode = if force { Mode::Force } else { Mode::Write };
    let clean = sync::sync(&project, mode, &mut out)?;

    if !clean {
        return Ok(ExitCode::FAILURE);
    }

    out.flush()?;
    run_skill(skills::INIT)
}

fn mode(check: bool, force: bool) -> Mode {
    if check {
        return Mode::Check;
    }

    if force {
        return Mode::Force;
    }

    Mode::Write
}

fn refresh(check: bool, project: bool, force: bool, out: &mut impl io::Write) -> Result<ExitCode> {
    let here = Project::new(&here()?);
    let sync_mode = mode(check, force);

    if project {
        if !here.exists() {
            writeln!(out)?;
            writeln!(out, "  this directory has no `.hod`; run `hod init` first")?;
            writeln!(out)?;
            return Ok(ExitCode::FAILURE);
        }

        return Ok(if sync::sync(&here, sync_mode, out)? {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        });
    }

    let binary = update::update(check, out)?;

    if !here.exists() {
        return Ok(binary);
    }

    let clean = sync::sync(&here, sync_mode, out)?;

    Ok(if binary != ExitCode::SUCCESS {
        binary
    } else if clean {
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

    agent::start(agent::find()?, &format!("/{name}"))
}
