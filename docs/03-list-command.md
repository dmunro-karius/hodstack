# `hod list`

## What's missing

There is no way to see which skills `hod` knows about locally — a user has
to already know a skill's name to run it.

## What the remote repo does

`cli/src/list.rs` (full file):

```rust
use std::io::Write;
use std::process::ExitCode;

use anyhow::Result;

use crate::project::Project;
use crate::skills::{self, Skill};

pub fn list(project: &Project, out: &mut impl Write) -> Result<ExitCode> {
    let shipped = skills::shipped();
    let mine = project.skills()?;

    writeln!(out)?;

    if shipped.is_empty() && mine.is_empty() {
        writeln!(out, "  No skill is installed. Run `hod update`.")?;
        writeln!(out)?;
        return Ok(ExitCode::SUCCESS);
    }

    let width = width(shipped.iter().chain(mine.iter()));

    group(out, "USER SKILLS", shipped.iter().filter(|skill| skill.front().user), width)?;
    group(out, "MODEL SKILLS", shipped.iter().filter(|skill| !skill.front().user), width)?;
    group(out, "PROJECT SKILLS", mine.iter(), width)?;

    Ok(ExitCode::SUCCESS)
}

fn sentence(description: &str) -> String {
    match description.split_once(". ") {
        Some((first, _)) => format!("{first}."),
        None => description.to_owned(),
    }
}

fn width<'a>(skills: impl Iterator<Item = &'a Skill>) -> usize {
    skills.map(|skill| skill.name.len()).max().unwrap_or(0).max(11)
}

fn group<'a>(
    out: &mut impl Write,
    heading: &str,
    skills: impl Iterator<Item = &'a Skill>,
    width: usize,
) -> Result<()> {
    let mut wrote = false;

    for skill in skills {
        if !wrote {
            writeln!(out, "{heading}")?;
            wrote = true;
        }

        let front = skill.front();
        writeln!(out, "  {:width$}  {}", front.name, sentence(&front.description))?;
    }

    if wrote {
        writeln!(out)?;
    }

    Ok(())
}
```

Sample output (from `cli/tests/cli.rs`, `list_names_each_skill_of_the_program_and_of_the_project`):

```
USER SKILLS
  deps-upgrade  Raise each dependency of this project to a newer version and keep the tests green.
  init          Write the intention of this project in `.hod/PROJECT.md`.

PROJECT SKILLS
  deploy        Deploy this project.
```

## Design points worth keeping

- Three groups, printed in this fixed order, each skipped entirely (no
  empty heading) when it has no members: **USER SKILLS** (shipped skills
  where `front().user` is true, i.e. `disable-model-invocation: true` in
  the skill's front matter — the user picks these explicitly), **MODEL
  SKILLS** (shipped skills the model can invoke on its own), **PROJECT
  SKILLS** (from `project.skills()`, i.e. `.hod/skills/`).
- Column alignment: name column width is `max(longest skill name, 11)` —
  the floor of 11 keeps the layout from looking cramped when every skill
  name is short.
- Description is truncated to its first sentence (`sentence()` splits on
  `". "`) so a long `description` field in a skill's front matter doesn't
  blow out the line.
- Empty state has its own message rather than an empty list: `No skill is
  installed. Run \`hod update\`.`
- This depends on `skills::shipped()` and `project.skills()` already
  existing and returning types with a `.front()` accessor exposing `name`,
  `description`, and `user: bool`. Compare against this repo's existing
  `src/skills.rs` / `src/front.rs` / `src/project.rs` — the remote's
  `cli/src/skills.rs`, `cli/src/front.rs`, and `cli/src/project.rs` are the
  same modules with more surface area (this repo's versions already diverge
  in content per the diff that produced this doc set), so treat those three
  remote files as the reference if `Skill`/`Front`/`Project` need new
  fields to support this.

## Implementation notes for this repo

- Requires `01-clap-cli-parsing.md` done first (`hod list` needs to be a
  `Command::List` variant).
- Add `mod list;` and wire `Command::List => list::list(&Project::new(&here()?), &mut out)`
  in the dispatch (see `01-clap-cli-parsing.md`'s `run()` excerpt).
- No new dependencies.
