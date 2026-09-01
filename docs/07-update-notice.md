# Background "a newer hod is available" notice

## What's missing

No awareness at all of whether the installed `hod` is out of date unless the
user thinks to run an update command manually.

## What the remote repo does

`cli/src/update.rs`'s `notice()`, called from `cli/src/lib.rs::run()` after
most commands finish (skipped for `update`/`completions` — see the `asks`
variable in `01-clap-cli-parsing.md`'s `run()` excerpt):

```rust
pub fn notice(err: &mut impl Write) {
    if env::var_os("HOD_NO_UPDATE_CHECK").is_some() {
        return;
    }
    if env::var_os("CI").is_some() {
        return;
    }
    if !io::stderr().is_terminal() {
        return;
    }

    let Some(running) = running() else {
        return;
    };
    let Some(path) = cache() else {
        return;
    };

    let known = fs::read_to_string(&path).ok();

    if let Some(build) = known.as_deref().and_then(|text| Build::read(text).ok()) {
        if build.commit != running {
            let _ = writeln!(err, "  {DIM}A newer hod is available. Run `hod update`.{DIM:#}\n");
        }
    }

    if fresh(&path) {
        return;
    }

    if !touch(&path, known.unwrap_or_default().as_str()) {
        return;
    }

    let _ = spawn_check();
}
```

Supporting pieces:

```rust
fn cache() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else {
        env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
    };
    Some(base?.join("hod").join("update"))
}

fn fresh(path: &Path) -> bool {
    let Ok(data) = fs::metadata(path) else { return false };
    let Ok(time) = data.modified() else { return false };
    match time.elapsed() {
        Ok(age) => age < DAY, // DAY = Duration::from_secs(60 * 60 * 24)
        Err(_) => true,
    }
}

fn spawn_check() -> Result<()> {
    let program = env::current_exe()?;
    Command::new(program)
        .args(["update", "--check"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}
```

## Design points worth keeping

- **The notice never blocks a command.** It reads a locally cached
  `~/.cache/hod/update` file (or `%LOCALAPPDATA%\hod\update` on Windows) —
  written by the *previous* run — and only prints a one-line dim hint if
  that cached "newest known build" disagrees with what's currently running.
  It does not make a network call inline.
- **The network call is detached.** If the cache file is more than a day
  old (`fresh()` returns false), it re-writes its own mtime immediately
  (`touch`, to debounce concurrent processes from all spawning checks at
  once) and then `spawn()`s a **detached background process**
  (`hod update --check`, stdio nulled) to refresh the cache for *next*
  time. The current invocation never waits on it.
- **Three opt-outs, checked in this order**: `HOD_NO_UPDATE_CHECK` env var,
  `CI` env var (any value — don't ping update servers from CI), and "is
  stderr actually a terminal" (`IsTerminal`, from `std::io`) — no point
  printing a notice into a log file or pipe no human will read live.
- **Fails silently everywhere** — every fallible step here uses `let _ =`
  or early-returns on `None`, deliberately: a failed update-notice check
  must never surface as an error to the user or affect the command's exit
  code.

## Implementation notes for this repo

- Depends on `04-update-command.md` for `Build::read`, the `running()` /
  `HOD_COMMIT` mechanism, and the cache file format (`"<version>
  <commit>\n"`) — implement that doc first, since `notice()` reads the same
  cache file that `update()`/`remember()` write.
- No new dependencies beyond what doc 4 already needs (`anstyle` for
  `DIM`).
- `IsTerminal` is `std::io::IsTerminal`, stable since Rust 1.70 — no crate
  needed.
