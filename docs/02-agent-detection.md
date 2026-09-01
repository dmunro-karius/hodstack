# Detect and launch a coding agent

## What's missing

Locally, whatever runs a skill (`cmd_run_skill` in `src/main.rs`) presumably
shells out to one fixed program. There's no concept of "which coding agent
is installed on this machine" — it can't run against Claude Code, Codex,
Cursor, opencode, or Gemini interchangeably.

## What the remote repo does

`cli/src/agent.rs` (full file, ~120 lines excluding tests) is a small,
self-contained module:

```rust
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use anyhow::{Context as _, Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opening {
    Positional,
    Flag(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Agent {
    pub binary: &'static str,
    opening: Opening,
}

const AGENTS: [Agent; 5] = [
    Agent { binary: "claude", opening: Opening::Positional },
    Agent { binary: "codex", opening: Opening::Positional },
    Agent { binary: "cursor-agent", opening: Opening::Positional },
    Agent { binary: "opencode", opening: Opening::Flag("--prompt") },
    Agent { binary: "gemini", opening: Opening::Flag("-i") },
];

pub fn find() -> Result<Agent> {
    if let Some(named) = env::var_os("HOD_AGENT") {
        let named = named.to_string_lossy().into_owned();

        let Some(agent) = AGENTS.iter().find(|agent| agent.binary == named) else {
            bail!("HOD_AGENT names `{named}`; hod starts {}", known())
        };

        return Ok(*agent);
    }

    let Some(agent) = AGENTS.iter().find(|agent| on_path(agent.binary).is_some()) else {
        bail!("no coding agent is on PATH; hod starts {}", known())
    };

    Ok(*agent)
}

pub fn start(agent: Agent, opening: &str) -> Result<ExitCode> {
    let mut command = Command::new(agent.binary);

    match agent.opening {
        Opening::Positional => command.arg(opening),
        Opening::Flag(flag) => command.arg(flag).arg(opening),
    };

    let status = command
        .status()
        .with_context(|| format!("cannot start `{}`", agent.binary))?;

    let Some(code) = status.code() else {
        return Ok(ExitCode::FAILURE);
    };

    Ok(match u8::try_from(code) {
        Ok(code) => ExitCode::from(code),
        Err(_) => ExitCode::FAILURE,
    })
}

fn known() -> String {
    AGENTS.iter().map(|agent| agent.binary).collect::<Vec<_>>().join(", ")
}

fn on_path(binary: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;

    for dir in env::split_paths(&path) {
        for name in names(binary) {
            let file = dir.join(name);
            if file.is_file() {
                return Some(file);
            }
        }
    }

    None
}

#[cfg(windows)]
fn names(binary: &str) -> Vec<OsString> {
    env::var("PATHEXT")
        .unwrap_or_else(|_| ".EXE;.CMD;.BAT".to_owned())
        .split(';')
        .map(|extension| OsString::from(format!("{binary}{extension}")))
        .collect()
}

#[cfg(not(windows))]
fn names(binary: &str) -> Vec<OsString> {
    vec![OsString::from(binary)]
}
```

## Design points worth keeping

- **`HOD_AGENT` env var override.** If set, it must name a known agent or
  `find()` fails loudly (`HOD_AGENT names \`nope\`; hod starts claude, codex,
  cursor-agent, opencode, gemini`) rather than silently falling back to
  autodetection.
- **First match on `PATH` wins**, in the fixed `AGENTS` table order —
  `claude` first. Not "all installed agents", not alphabetical.
- **Two invocation shapes.** Most agents take the prompt positionally
  (`claude "/skill-name"`); `opencode` wants `--prompt "/skill-name"` and
  `gemini` wants `-i "/skill-name"`. The `Opening` enum captures this so
  `start()` stays a two-line match rather than growing per-agent branches.
- **`on_path` does manual `PATH` scanning** rather than using a crate like
  `which`, and handles `PATHEXT` on Windows (`.EXE`/`.CMD`/`.BAT`) vs. a bare
  binary name elsewhere via `#[cfg(windows)]`. No new dependency needed.
- **Exit code passthrough.** `start()` forwards the child's real exit code
  (clamped to `u8`, `ExitCode::FAILURE` if the process was killed by a
  signal and reports no code) — the caller's shell sees the agent's own
  success/failure, not a generic 0/1.
- The opening prompt is always `/<skill-name>` — see
  `05-init-flow.md` and `03-list-command.md` for how `skill.name` is
  resolved before being handed to `agent::start`.

## Implementation notes for this repo

- Port `agent.rs` essentially unchanged into `src/agent.rs`; add `mod
  agent;` to `main.rs`/`lib.rs`.
- Wire it into wherever the local code currently invokes a fixed skill
  runner: replace that fixed `Command::new("claude")`-style call (or
  whatever it is) with `agent::start(agent::find()?, &format!("/{}",
  skill.name))`.
- No new crate dependency required — `agent.rs` only uses `std` + the
  `anyhow` this repo already has.
- The remote's tests for this module (also in `agent.rs`, `#[cfg(test)]
  mod tests`) are worth copying too — they check that the error message
  from `known()` names every agent, that a nonexistent binary reports
  absent from `on_path`, and that `AGENTS[0]` is `claude` with
  `Opening::Positional` (a regression guard on ordering, since ordering is
  the whole autodetection policy).
