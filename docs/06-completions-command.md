# `hod completions <shell>` and styled `--help`

## What's missing

No shell completion generation, and `--help` output is whatever `clap`'s
(or the hand-rolled `print_help`'s) default rendering looks like — no color,
no custom layout, no link to the repo, no path to the running binary.

## What the remote repo does

### Completions subcommand

Declared in `cli/src/cli.rs` (see `01-clap-cli-parsing.md`):

```rust
#[command(about = "Print a shell completion script")]
Completions {
    #[arg(help = "The shell that receives the script")]
    shell: clap_complete::Shell,
},
```

Dispatched in `cli/src/lib.rs`:

```rust
Command::Completions { shell } => Ok(completions(shell)),
```

```rust
#[expect(
    clippy::unimplemented,
    reason = "skeleton: `hod completions` has no body today"
)]
fn completions(_shell: clap_complete::Shell) -> ExitCode {
    unimplemented!("`hod completions` has no body today")
}
```

**Important**: as of this diff, the remote repo has only wired the
*command* through clap — the actual generation body is still
`unimplemented!()`. If implementing this for real (not just matching
upstream), use `clap_complete::generate`:

```rust
fn completions(shell: clap_complete::Shell) -> ExitCode {
    let mut command = command();
    let name = command.get_name().to_owned();
    clap_complete::generate(shell, &mut command, name, &mut io::stdout());
    ExitCode::SUCCESS
}
```

There's a corresponding test in `cli/tests/cli.rs` confirming the current
(unimplemented) behavior:

```rust
#[test]
fn a_command_without_a_body_fails() {
    Command::new(HOD).args(["completions", "zsh"]).assert().failure();
}
```

— replace/remove that test once a real body is added.

### Styled help template

`cli/src/help.rs` (full file):

```rust
use std::env;

use clap::builder::styling::{AnsiColor, Color, Style, Styles};

const BOLD: Style = Style::new().bold();
const DIM: Style = Style::new().dimmed();

pub const STYLES: Styles = Styles::styled()
    .header(BOLD)
    .usage(BOLD)
    .literal(BOLD)
    .placeholder(DIM)
    .error(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red))).bold())
    .invalid(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow))).bold())
    .valid(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))))
    .context(DIM);

pub fn template() -> String {
    format!(
        "\n\
         \n\
         {BOLD}{{name}}{BOLD:#} {DIM}{{version}} — {{about}}{DIM:#}\n\
         \n\
         {BOLD}USAGE{BOLD:#}\n\
         \x20\x20{{usage}}\n\
         \n\
         {BOLD}COMMANDS{BOLD:#}\n\
         {{subcommands}}\n\
         \n\
         {BOLD}OPTIONS{BOLD:#}\n\
         {{options}}\n\
         \n\
         {DIM}github.com/hodstack/hodstack{DIM:#}{}",
        binary()
    )
}

fn binary() -> String {
    match env::current_exe() {
        Ok(path) => format!("\n{DIM}{}{DIM:#}", path.display()),
        Err(_) => String::new(),
    }
}
```

Applied in `cli/src/lib.rs`:

```rust
pub fn command() -> clap::Command {
    Cli::command().version(version()).help_template(help::template())
}
```

`--version` string embeds the build's commit when available (see
`04-update-command.md`'s `Build`/`HOD_COMMIT` concept):

```rust
fn version() -> &'static str {
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
```

Test coverage from `cli/tests/cli.rs` worth porting:

```rust
#[test]
fn the_root_screen_names_the_path_of_this_binary() {
    let mut command = hod::command();
    let binary = std::env::current_exe().unwrap();
    let screen = command.render_help().to_string();
    assert!(screen.trim_end().ends_with(&binary.display().to_string()));
}

#[test]
fn the_root_screen_carries_a_style() {
    let mut command = hod::command();
    let screen = command.render_help().ansi().to_string();
    assert!(screen.contains('\u{1b}'), "the screen carries no style");
}
```

Plus a full-screen snapshot test using `snapbox` (`file!["snapshots/root.txt"]`)
— see `10-testing-infrastructure.md`.

## Design points worth keeping

- The help screen prints the *path of the running binary* at the bottom —
  useful for a user debugging "which `hod` am I actually running" when
  multiple installs exist (cargo-installed vs. npm vs. homebrew, etc.).
- Colors are applied via clap's `Styles` builder, not raw ANSI codes
  scattered through format strings — one place (`STYLES`) controls the
  whole palette.
- `anstream` (already a dependency per `01-clap-cli-parsing.md`'s `run()`
  excerpt using `anstream::stdout()`/`anstream::stderr()`) auto-detects
  whether the output is a terminal and strips styling for pipes/redirects —
  don't hand-roll that detection.

## Implementation notes for this repo

- Requires `01-clap-cli-parsing.md` first.
- New dependencies: `clap_complete` (already listed in doc 1),
  `anstyle` (also needed by `04-update-command.md`).
- Decide up front whether to ship the completions body as
  `unimplemented!()` like upstream currently does, or implement it for
  real via `clap_complete::generate` — either is a smaller, standalone
  follow-up once the `clap` migration (doc 1) lands.
