# Migrate argument parsing to `clap`

## What's missing

`src/main.rs` parses `env::args()` by hand:

```rust
let args: Vec<String> = env::args().skip(1).collect();

let result = match args.first().map(String::as_str) {
    None | Some("-h") | Some("--help") => {
        print_help();
        Ok(ExitCode::SUCCESS)
    }
    Some("init") => cmd_init(args.iter().any(|arg| arg == "--force")),
    Some(skill) => cmd_run_skill(skill),
};
```

This only understands `init` (with a bespoke `--force` scan) or a skill
name. It has no real flag parsing, no per-command help, no shell
completions, and no structured error on bad input (e.g. two positional
args). Every other feature doc in this folder (`list`, `update`,
`completions`, styled `--help`) assumes a `clap`-based command tree exists
first — do this one first.

## What the remote repo does

`cli/src/cli.rs` defines the whole CLI surface as a `clap::Parser`:

```rust
use clap::{Parser, Subcommand};

use crate::help;

#[derive(Debug, Parser)]
#[command(
    name = "hod",
    version,
    about = "Hodstack makes coding agents more productive.",
    long_about = None,
    override_usage = "hod <skill>\n  hod <command> [options]",
    args_conflicts_with_subcommands = true,
    disable_help_subcommand = true,
    styles = help::STYLES,
    term_width = 0,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(value_name = "skill", help = "The name of the skill")]
    pub skill: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Write the files of this project, then start the skill init")]
    Init,

    #[command(about = "List the installed skills")]
    List,

    #[command(about = "Install the newest build of hod and write the project files")]
    Update {
        #[arg(long, help = "Report each change without a write of it")]
        check: bool,

        #[arg(long, help = "Write the project files without an installation of hod")]
        project: bool,

        #[arg(
            long,
            conflicts_with = "check",
            help = "Write over a project file that this program does not own"
        )]
        force: bool,
    },

    #[command(about = "Print a shell completion script")]
    Completions {
        #[arg(help = "The shell that receives the script")]
        shell: clap_complete::Shell,
    },
}
```

Key trick: `args_conflicts_with_subcommands = true` plus a top-level
optional positional `skill` field lets `hod <skill>` and `hod <command>`
coexist under one parser — `hod deps-upgrade` binds `skill`, `hod list`
binds `command`, and clap rejects `hod list deps-upgrade` for you.

`cli/src/lib.rs` builds the `clap::Command`, applies custom help styling and
a custom help template (see doc `06-completions-command.md` and the
`help.rs` excerpt below), and dispatches:

```rust
pub fn command() -> clap::Command {
    Cli::command()
        .version(version())
        .help_template(help::template())
}

pub fn run() -> Result<ExitCode> {
    let matches = command().get_matches();
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(error) => error.exit(),
    };

    let mut out = anstream::stdout().lock();

    let Some(command) = cli.command else {
        let Some(skill) = cli.skill else {
            command().print_help()?;
            return Ok(ExitCode::SUCCESS);
        };

        let code = start(&skill)?;
        out.flush()?;
        update::notice(&mut anstream::stderr().lock());
        return Ok(code);
    };

    let code = match command {
        Command::Init => setup(&mut out),
        Command::List => list::list(&Project::new(&here()?), &mut out),
        Command::Update { check, project, force } => refresh(check, project, force, &mut out),
        Command::Completions { shell } => Ok(completions(shell)),
    }?;

    Ok(code)
}
```

Note `command()` builds the `clap::Command` fresh each call (used both to
`get_matches()` and to `print_help()` on the no-args path) rather than
caching it — cheap enough, and it sidesteps borrow issues with a shared
`OnceLock<clap::Command>`.

## Dependencies to add

From `cli/Cargo.toml`:

```toml
clap = { version = "4.6", features = ["derive"] }
clap_complete = "4.6"
```

## Implementation notes for this repo

- Add `mod cli;` to `src/main.rs`, move the enum/struct above into
  `src/cli.rs`.
- `src/main.rs` shrinks to essentially just calling a `run()` function (see
  `cli/src/lib.rs` `run()` above) and mapping the `Result` to an
  `ExitCode`/error report — compare with the remote's actual `main.rs`,
  which is 8 lines:

  ```rust
  use std::process::ExitCode;

  fn main() -> ExitCode {
      match hod::run() {
          Ok(code) => code,
          Err(error) => hod::report(&error),
      }
  }
  ```

  This only works if the rest of the logic lives in a library crate
  (`lib.rs`) rather than in `main.rs` — worth doing anyway since it's what
  lets `cli/tests/cli.rs` call `hod::init(...)` directly in-process for fast
  unit-style assertions alongside the slower `Command::new(HOD)` integration
  tests (see `10-testing-infrastructure.md`).
- Don't implement `Command::Completions` for real yet if you're not ready —
  the remote itself stubs it with `unimplemented!()` (see
  `06-completions-command.md`). Get `Init` working under the new parser
  first, since `cmd_init`'s `--force` scan already exists locally and is the
  smallest thing to port.
- `error.exit()` on a `Cli::from_arg_matches` failure is what gives clap's
  standard exit code 2 for usage errors — don't swallow that `Err` path.
