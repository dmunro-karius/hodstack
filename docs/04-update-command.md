# `hod update` — self-update the binary and re-sync project files

## What's missing

There's no way for `hod` to update itself, and no `--check`/`--project`/
`--force` distinction for re-running the file sync that `init` does.

## What the remote repo does

`cli/src/update.rs` (440 lines total; reproduced here minus the `#[cfg(test)]`
block — see the file at that path in the remote repo for the unit tests,
which are good coverage to port too) handles the binary self-update:

```rust
use std::env;
use std::fs;
use std::io::{self, IsTerminal as _, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Duration;

use anstyle::Style;
use anyhow::{Context as _, Result, anyhow, bail};
use sha2::{Digest as _, Sha256};

const BOLD: Style = Style::new().bold();
const DIM: Style = Style::new().dimmed();
const RELEASE: &str = "https://github.com/hodstack/hodstack/releases/latest/download";
const DAY: Duration = Duration::from_secs(60 * 60 * 24);

#[derive(Debug, PartialEq, Eq)]
struct Build {
    version: String,
    commit: String,
}

impl Build {
    fn read(text: &str) -> Result<Self> {
        let mut fields = text.split_whitespace();
        let version = fields.next().ok_or_else(|| anyhow!("version.txt names no version"))?;
        let commit = fields.next().ok_or_else(|| anyhow!("version.txt names no commit"))?;
        Ok(Self { version: version.to_owned(), commit: commit.to_owned() })
    }

    fn write(&self) -> String {
        format!("{} {}\n", self.version, self.commit)
    }

    fn label(&self) -> String {
        format!("{} ({})", self.version, short(&self.commit))
    }
}

pub fn update(check: bool, out: &mut impl Write) -> Result<ExitCode> {
    let path = env::current_exe().context("cannot read the path of this program")?;
    let binary = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());

    let newest = newest()?;
    remember(&newest);

    writeln!(out)?;

    if running() == Some(newest.commit.as_str()) {
        writeln!(out, "  {BOLD}Current{BOLD:#}  {}", newest.label())?;
        writeln!(out, "           {DIM}{}{DIM:#}", path.display())?;
        writeln!(out)?;
        return Ok(ExitCode::SUCCESS);
    }

    if check {
        writeln!(out, "  {BOLD}Newest{BOLD:#}   {}", newest.label())?;
        writeln!(out, "           Run `hod update` to install it.")?;
        writeln!(out)?;
        return Ok(ExitCode::SUCCESS);
    }

    install(&binary, &newest)?;

    writeln!(out, "  {BOLD}Updated{BOLD:#}  {} → {}", crate::version(), newest.label())?;
    writeln!(out, "           {DIM}{}{DIM:#}", path.display())?;
    writeln!(out)?;

    Ok(ExitCode::SUCCESS)
}
```

The install path: fetch `version.txt` (`"<crate-version> <commit>\n"`) and
`hod-<target>.tar.gz` + `checksums.txt` from the GitHub release, verify the
SHA-256 of the archive against `checksums.txt` before touching anything,
extract with `tar`, `chmod 755` on Unix, then atomically swap the running
binary:

```rust
fn asset() -> Result<&'static str> {
    let target = match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "aarch64") => "aarch64-unknown-linux-musl",
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        (os, arch) => bail!("no build for {os} {arch}"),
    };
    Ok(target)
}

fn download(work: &Path, file: &str) -> Result<()> {
    let address = address();
    let archive = work.join(file);
    let checksums = work.join("checksums.txt");

    fetch(&format!("{address}/{file}"), &archive)?;
    fetch(&format!("{address}/checksums.txt"), &checksums)?;

    let checksums = fs::read_to_string(&checksums).context("cannot read checksums.txt")?;
    let wanted = wanted_sum(&checksums, file)?;
    let found = sum(&archive)?;

    if wanted != found {
        bail!("the checksum of {file} does not agree with checksums.txt");
    }

    extract(&archive, work)
}

fn swap(new: &Path, binary: &Path, parent: &Path, name: &Path) -> Result<()> {
    let old = parent.join(format!(".{}.old", name.display()));

    let _ = fs::remove_file(&old);
    fs::rename(binary, &old).with_context(|| format!("cannot move `{}`", binary.display()))?;

    match fs::rename(new, binary) {
        Ok(()) => {
            let _ = fs::remove_file(&old);
            Ok(())
        }
        Err(error) => {
            let _ = fs::rename(&old, binary);
            Err(error).with_context(|| format!("cannot write `{}`", binary.display()))
        }
    }
}
```

`fetch()` shells out to `curl` first, falls back to `wget`, and fails with
`this computer has no curl and no wget` if neither exists — no HTTP client
crate dependency (keeps the binary small and avoids TLS-stack bloat).

## `hod update` vs `hod update --project` vs `hod update --check`

From `cli/src/lib.rs`:

```rust
fn mode(check: bool, force: bool) -> Mode {
    if check {
        return Mode::Check;
    }
    if force {
        return Mode::Force;
    }
    Mode::Write
}

fn refresh(check: bool, project: bool, force: bool, out: &mut impl Write) -> Result<ExitCode> {
    let here = Project::new(&here()?);
    let mode = mode(check, force);

    if project {
        if !here.exists() {
            writeln!(out)?;
            writeln!(out, "  This directory has no `.hod`. Run `hod init` first.")?;
            writeln!(out)?;
            return Ok(ExitCode::FAILURE);
        }
        return sync::sync(&here, mode, out);
    }

    let binary = update::update(check, out)?;

    if !here.exists() {
        return Ok(binary);
    }

    let files = sync::sync(&here, mode, out)?;

    Ok(if binary == ExitCode::SUCCESS { files } else { binary })
}
```

So plain `hod update` does **both**: update the `hod` binary itself, then
(if the current directory is a project, i.e. has `.hod/`) re-run the file
sync (`sync::sync`, the same pass `init` runs — see `05-init-flow.md` and
the `sync`/`lock`/`ownership` terms in `12-ci-cd-and-repo-docs.md`'s
`CONTEXT.md` excerpt). `--project` skips the binary update and only does
the file sync (failing if there's no `.hod`). `--check` makes either path
report-only. `--force` (project sync only, `conflicts_with = "check"` per
the `clap` definition in `01-clap-cli-parsing.md`) allows overwriting a
project file the tool doesn't own (see the `ownership`/`lock` concepts —
`Ours` vs `Theirs` — in doc 12).

## Design points worth keeping

- **Checksum verification before any write.** Never extract/install
  without confirming the SHA-256 against `checksums.txt` first — a mismatch
  is a hard `bail!`, and the running binary is left untouched.
- **Atomic-ish swap**: rename the current binary aside to `.<name>.old`,
  rename the new one into place, then delete the `.old` — and if the second
  rename fails, rename `.old` back so the machine is never left without a
  working `hod`.
- **`HOD_RELEASE_URL` env var** overrides the release base URL (used by the
  remote's own tests to point at a local `file://` fixture — see
  `10-testing-infrastructure.md`).
- Update caches the newest known build info under a per-OS cache dir
  (`XDG_CACHE_HOME`/`~/.cache` on Unix, `LOCALAPPDATA` on Windows) at
  `hod/update`, reused by the background notice feature — see
  `07-update-notice.md`.

## Implementation notes for this repo

- Requires `01-clap-cli-parsing.md` (the `Command::Update { check, project,
  force }` variant) and an existing `sync`/`lock`/`project` module set —
  this repo already has `src/sync.rs`, `src/lock.rs`, `src/project.rs`, so
  the `--project` half of this is mostly wiring existing sync logic behind
  a new `Mode::Check`/`Mode::Force`/`Mode::Write` enum if one doesn't
  already exist locally. Diff this repo's `src/sync.rs` against the
  remote's `cli/src/sync.rs` for the exact `Mode` shape expected.
- The binary self-update half (`update.rs`) is independent and can be
  built/tested without a real GitHub release — point `HOD_RELEASE_URL` at a
  local directory of fixtures, exactly as `cli/tests/cli.rs` does (see
  `10-testing-infrastructure.md`).
- New dependency: `sha2` (this repo already has it in `Cargo.toml`).
  `anstyle` is also needed for the `BOLD`/`DIM` output styling (see
  `06-completions-command.md` for the same dependency used by `help.rs`).
- This feature has a hard dependency on `08-installers-packaging.md`'s
  release layout (`version.txt`, `hod-<target>.tar.gz`, `checksums.txt`
  published together) — the update command and the release pipeline must
  agree on this file layout, so implement/adjust them together.
