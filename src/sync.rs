use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context as _, Result};

use crate::lock::{Lock, Owner};
use crate::project::{self, Project};

const IGNORED: &str = "/.claude/skills/";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Outcome {
    Kept,
    Created,
    Updated,
    Removed,
    Skipped,
}

struct Unit {
    label: String,
    files: Vec<(String, String)>,
}

/// Writes `AGENTS.md`, `CLAUDE.md`, and every installed skill into `.claude/skills/`,
/// gating each write on `.hod/lock` ownership. Returns `true` if nothing was skipped.
pub fn sync(project: &Project, force: bool, out: &mut impl Write) -> Result<bool> {
    let rules = project.rules()?;
    let skills = project.installed_skills()?;
    let units = plan(&rules, &skills);

    let lock = Lock::read(&project.lock_path());
    let mut next = Lock::default();
    let mut clean = true;

    for unit in &units {
        let outcome = keep(project, &lock, &mut next, unit, force)?;

        report(out, outcome, &unit.label)?;

        if outcome == Outcome::Skipped {
            writeln!(
                out,
                "           this file is yours; rerun `hod init --force` to write over it"
            )?;
            clean = false;
        }
    }

    for label in gone(project, &lock, &next, &units)? {
        report(out, Outcome::Removed, &label)?;
    }

    if let Some(outcome) = ignore(project)? {
        report(out, outcome, ".gitignore")?;
    }

    next.write(&project.lock_path())?;

    Ok(clean)
}

fn plan(rules: &[project::Rule], skills: &[crate::skills::Skill]) -> Vec<Unit> {
    let mut units = vec![
        Unit {
            label: project::AGENTS.to_owned(),
            files: vec![(project::AGENTS.to_owned(), project::agents_md(rules))],
        },
        Unit {
            label: project::CLAUDE.to_owned(),
            files: vec![(project::CLAUDE.to_owned(), project::IMPORT.to_owned())],
        },
    ];

    for skill in skills {
        units.push(Unit {
            label: format!("{}/{}", project::SKILLS_CLIENT, skill.name),
            files: skill
                .files
                .iter()
                .map(|(file, text)| {
                    (
                        format!("{}/{}/{file}", project::SKILLS_CLIENT, skill.name),
                        text.clone(),
                    )
                })
                .collect(),
        });
    }

    units
}

fn keep(project: &Project, lock: &Lock, next: &mut Lock, unit: &Unit, force: bool) -> Result<Outcome> {
    let mut outcome = Outcome::Kept;

    for (file, text) in &unit.files {
        let path = project.path(file);
        let found = fs::read(&path).ok();
        let state = lock.state(file, found.as_deref());

        if state == Owner::Theirs && !force {
            outcome = outcome.max(Outcome::Skipped);
            continue;
        }

        next.keep(file, crate::lock::sum_of(text.as_bytes()));

        if found.as_deref() == Some(text.as_bytes()) {
            continue;
        }

        outcome = outcome.max(if state == Owner::Absent {
            Outcome::Created
        } else {
            Outcome::Updated
        });

        write(&path, text)?;
    }

    Ok(outcome)
}

fn gone(project: &Project, lock: &Lock, next: &Lock, units: &[Unit]) -> Result<Vec<String>> {
    let mut labels = Vec::new();

    for file in lock.files() {
        if next.holds(file) {
            continue;
        }

        let path = project.path(file);
        let found = fs::read(&path).ok();

        if lock.state(file, found.as_deref()) != Owner::Ours {
            continue;
        }

        fs::remove_file(&path).with_context(|| format!("cannot remove `{}`", path.display()))?;

        let label = label(file);

        if units.iter().any(|unit| unit.label == label) {
            continue;
        }

        if !labels.contains(&label) {
            labels.push(label);
        }
    }

    for label in &labels {
        prune(&project.path(label));
    }

    Ok(labels)
}

fn label(file: &str) -> String {
    if let Some(rest) = file.strip_prefix(&format!("{}/", project::SKILLS_CLIENT)) {
        if let Some(name) = rest.split('/').next() {
            return format!("{}/{name}", project::SKILLS_CLIENT);
        }
    }

    file.to_owned()
}

fn prune(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            prune(&path);
        }
    }

    let _ = fs::remove_dir(dir);
}

fn ignore(project: &Project) -> Result<Option<Outcome>> {
    if !project.path(".git").exists() {
        return Ok(None);
    }

    let path = project.path(".gitignore");
    let found = fs::read_to_string(&path).unwrap_or_default();

    if found.lines().any(|line| line.trim() == IGNORED) {
        return Ok(None);
    }

    let mut text = found.clone();

    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }

    if !text.is_empty() {
        text.push('\n');
    }

    text.push_str(IGNORED);
    text.push('\n');

    write(&path, &text)?;

    Ok(Some(if found.is_empty() {
        Outcome::Created
    } else {
        Outcome::Updated
    }))
}

fn write(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot write in `{}`", parent.display()))?;
    }

    fs::write(path, text).with_context(|| format!("cannot write `{}`", path.display()))
}

fn report(out: &mut impl Write, outcome: Outcome, label: &str) -> Result<()> {
    let word = match outcome {
        Outcome::Kept => "kept",
        Outcome::Created => "created",
        Outcome::Updated => "updated",
        Outcome::Removed => "removed",
        Outcome::Skipped => "skipped",
    };

    writeln!(out, "  {word:<8}{label}")?;

    Ok(())
}
