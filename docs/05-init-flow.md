# `hod init`: refuse to clobber, then launch the agent

## What's missing

It's unclear whether the local `cmd_init` already refuses to overwrite an
existing `AGENTS.md`/`CLAUDE.md`, but it does not hand off to a coding agent
afterward — the remote's `init` both protects existing files *and* opens the
detected agent into a follow-up "fill in your project intent" skill in one
command.

## What the remote repo does

`cli/src/init.rs` (full file):

```rust
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context as _, Result};

use crate::project::{self, Project};
use crate::sync::{self, Mode};

pub fn init(dir: &Path, out: &mut impl Write) -> Result<ExitCode> {
    let project = Project::new(dir);

    for name in [project::AGENTS, project::CLAUDE] {
        let path = project.path(name);
        let exists = path
            .try_exists()
            .with_context(|| format!("cannot read `{}`", path.display()))?;

        if exists {
            writeln!(out, "  {name} already exists. Nothing was written.")?;
            writeln!(out, "  Move its text to `{}`, then run `hod init` again.", project::INTENTION)?;
            return Ok(ExitCode::FAILURE);
        }
    }

    sync::sync(&project, Mode::Write, out)
}
```

`cli/src/lib.rs`'s `setup()` is what wraps this with the agent hand-off:

```rust
fn setup(out: &mut impl Write) -> Result<ExitCode> {
    let code = init(&here()?, out)?;

    if code != ExitCode::SUCCESS {
        return Ok(code);
    }

    out.flush()?;
    start(skills::INIT)
}
```

`start(name)` (also in `lib.rs`) resolves a skill by name via
`project.skill(name)` and then calls `agent::start(agent::find()?,
&format!("/{}", skill.name))` — see `02-agent-detection.md`.

## Design points worth keeping

- **Fail closed, not partial-write.** Before writing anything, `init` checks
  *both* `AGENTS.md` and `CLAUDE.md` for existence and bails with
  `ExitCode::FAILURE` and no writes at all if either exists — it never
  writes one and skips the other. The error message tells the user exactly
  what to do (`Move its text to \`.hod/PROJECT.md\`, then run \`hod init\`
  again.`), not just that it failed.
- **`init` only ever writes once.** Confirmed by the remote's own test
  (`cli/tests/cli.rs`, `init_keeps_a_file_that_exists_and_writes_nothing`):
  if `AGENTS.md` pre-exists, `.hod` itself is never created either — no
  half-initialized project state.
- **Successful init immediately launches the agent** into the `init` skill
  (`skills::INIT`, i.e. `/init` as the opening prompt) — the point is that
  `hod init` isn't just "write some template files", it's "get the coding
  agent talking to the user about what this project *is*" right away, which
  is what fills in `.hod/PROJECT.md` with real content instead of the
  template placeholder. See the `intention` term in `12-ci-cd-and-repo-docs.md`.
- Note the ordering: `out.flush()` happens *before* `start(...)` — the
  sync's own status lines (`Created AGENTS.md`, etc.) must hit the
  terminal before the agent process takes over stdio.

## Implementation notes for this repo

- Requires `01-clap-cli-parsing.md` (`Command::Init`) and
  `02-agent-detection.md` (the `start(name)` hand-off).
- Compare this repo's existing `cmd_init` in `src/main.rs` against
  `project::init` above — if it doesn't already do the "check both files
  before writing either" guard, that's the main gap to close here; the
  agent hand-off is the second, separate gap.
- `project::AGENTS`, `project::CLAUDE`, `project::INTENTION` are constants
  this repo's `src/project.rs` should already define given it already
  writes `AGENTS.md`/`CLAUDE.md` — confirm the constant names match before
  reusing this snippet verbatim.
