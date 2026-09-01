# Testing infrastructure: snapshot/integration tests, supply-chain checks

## What's missing

No `tests/` directory, no integration test binary, and no supply-chain
tooling config (`cargo-deny`, `cargo-vet`) at all.

## What the remote repo does

### Integration tests (`cli/tests/cli.rs`, ~520 lines)

Built on `snapbox` (added as a dev-dependency: `snapbox = { version = "1.2",
features = ["cmd"] }`) which gives `Command::new(BIN).assert().success()...`
style assertions plus snapshot-file comparisons.

Two testing styles coexist in the same file:

**1. In-process, via the library crate** — fast, no subprocess spawn, used
for pure logic:

```rust
#[test]
fn the_root_screen_lists_each_command() {
    let mut command = hod::command();
    let screen = command.render_help().to_string();
    assert_data_eq!(screen, file!["snapshots/root.txt"]);
}

#[test]
fn init_writes_the_files_of_this_program_and_the_seed_of_the_user() {
    let dir = tempfile::tempdir().unwrap();
    let mut out = Vec::new();

    let code = hod::init(dir.path(), &mut out).unwrap();

    assert_eq!(code, std::process::ExitCode::SUCCESS);
    assert_eq!(
        fs::read_to_string(dir.path().join("AGENTS.md")).unwrap(),
        include_str!("../templates/rules.md")
    );
    assert!(dir.path().join(".hod/lock").is_file());
    assert!(dir.path().join(".claude/skills/init/SKILL.md").is_file());
}
```

This is only possible because `pub use crate::init::init;` and `pub fn
command() -> clap::Command` are exported from the library crate (`cli/src/
lib.rs`) — see `01-clap-cli-parsing.md`'s note about splitting logic out
of `main.rs` into `lib.rs` for exactly this reason.

**2. Real subprocess, via `Command::new(HOD)`** — for full end-to-end
behavior, including exit codes, stderr text, and env var handling:

```rust
const HOD: &str = env!("CARGO_BIN_EXE_hod");

#[test]
fn a_skill_that_is_absent_is_a_fault() {
    Command::new(HOD)
        .arg("nope")
        .assert()
        .failure()
        .stderr_eq("error: no skill has the name `nope`; run `hod list` to name each skill\n");
}
```

`CARGO_BIN_EXE_hod` is a Cargo-provided env var pointing at the just-built
`hod` binary — no manual path plumbing needed.

**Faking the update-server and coding-agent for tests** — both use the same
trick: point env vars at fixtures instead of the real network/`PATH`.

Fake release server (a `file://` URL, no HTTP server needed):

```rust
fn release(dir: &Path, body: &str, sound: bool) -> String {
    let name = if cfg!(windows) { "hod.exe" } else { "hod" };
    let file = format!("hod-{}.tar.gz", target());
    let staging = dir.join("staging");
    fs::create_dir_all(&staging).unwrap();
    fs::write(staging.join(name), body).unwrap();

    std::process::Command::new("tar")
        .arg("-czf").arg(dir.join(&file)).arg("-C").arg(&staging).arg(name)
        .status().unwrap();

    let sum = format!("{:x}", Sha256::digest(fs::read(dir.join(&file)).unwrap()));
    let sum = if sound { sum } else { "0".repeat(64) };

    fs::write(dir.join("checksums.txt"), format!("{sum}  {file}\n")).unwrap();
    fs::write(dir.join("version.txt"), format!("{NEWEST}\n")).unwrap();

    format!("file://{}", dir.display())
}

fn update(binary: &Path, home: &Path, address: &str) -> Command {
    Command::new(binary)
        .arg("update")
        .env("HOD_RELEASE_URL", address)
        .env("HOME", home)
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env("LOCALAPPDATA", home.join("cache"))
}

#[test]
fn update_keeps_this_build_when_the_checksum_does_not_agree() {
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let address = release(dir.path(), "the newest build", false); // sound=false -> bad checksum
    let binary = installed(home.path());
    let before = fs::read(&binary).unwrap();

    update(&binary, home.path(), &address)
        .assert()
        .failure()
        .stderr_eq(format!(
            "error: cannot install 9.9.9 (3f2b1c9): the checksum of hod-{}.tar.gz does not agree with checksums.txt\n",
            target()
        ));

    assert_eq!(fs::read(&binary).unwrap(), before); // binary untouched on failure
}
```

Fake coding agent (a shell script standing in for `claude`/`opencode`, on
`PATH` via `.env("PATH", &bin)`):

```rust
#[cfg(unix)]
fn agent(dir: &Path, binary: &str, record: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt as _;
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let file = bin.join(binary);
    fs::write(&file, format!("#!/bin/sh\nprintf '%s' \"$*\" > {}\n", record.display())).unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

#[cfg(unix)]
#[test]
fn a_skill_starts_the_agent_with_its_slash_command() {
    let dir = tempfile::tempdir().unwrap();
    let record = dir.path().join("record");
    let bin = agent(dir.path(), "claude", &record);

    Command::new(HOD)
        .arg("deps-upgrade")
        .current_dir(dir.path())
        .env("PATH", &bin)
        .env_remove("HOD_AGENT")
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&record).unwrap(), "/deps-upgrade");
}
```

This is the pattern to reuse for testing `02-agent-detection.md` and
`04-update-command.md` without ever hitting a real network or spawning a
real `claude`/`codex` process.

### Supply-chain / dependency hygiene

`cli/deny.toml` (`cargo-deny`):

```toml
[graph]
all-features = true

[advisories]
yanked = "deny"
ignore = []

[licenses]
allow = ["MIT", "Apache-2.0", "Unicode-3.0"]
confidence-threshold = 0.93

[bans]
multiple-versions = "warn"
wildcards = "deny"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

`cli/supply-chain/` holds `cargo-vet` config (`config.toml`,
`audits.toml`, `imports.lock`) — a manual/trusted-source audit trail for
every third-party crate in the dependency tree, checked in CI (see
`12-ci-cd-and-repo-docs.md`'s `vet` job).

`cargo-audit` (RustSec advisory DB) and `cargo-machete` (unused-dependency
detection) round out the checks, both wired as `cargo-make` tasks (see
`09-build-quality-lints.md`).

## Design points worth keeping

- Prefer the in-process style (`hod::init(...)`, `hod::command()`) for
  logic-heavy assertions — it's faster and gives direct access to return
  values, not just stdout text. Reserve subprocess (`Command::new(HOD)`)
  tests for things that are genuinely about process boundaries: exit
  codes, env var propagation, `PATH` lookup, real file writes from a fresh
  `current_dir`.
- Fixture fakes (fake release server via `file://`, fake agent via a shell
  script) keep tests hermetic — no network, no dependency on `claude`/
  `codex` actually being installed on the CI runner.
- `#[expect(clippy::unwrap_used, reason = "a test stops on a fault of its own")]`
  is how the remote's own `unwrap_used = "deny"` lint (see doc 9) is locally
  waived in test helper functions, with a required reason string — not a
  blanket `#[allow]`.

## Implementation notes for this repo

- Requires most of docs 1–7 to exist first, since these tests exercise
  `hod::init`, `hod::command()`, `hod list`, `hod update`, agent dispatch,
  etc. — write incrementally alongside each feature rather than all at
  once at the end.
- New dev-dependencies: `snapbox = { version = "1.2", features = ["cmd"] }`,
  and (implied by the temp-dir usage throughout) `tempfile`.
- `cargo-deny`/`cargo-vet`/`cargo-audit`/`cargo-machete` are independent of
  the Rust code changes — can be adopted at any time by copying
  `cli/deny.toml` and running `cargo vet init` to seed `supply-chain/`
  against this repo's actual (smaller) dependency tree.
