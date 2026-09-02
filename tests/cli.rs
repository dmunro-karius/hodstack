#![expect(clippy::expect_used, reason = "a test stops on a fault of its own")]
#![expect(clippy::panic, reason = "a test stops on a fault of its own")]

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};
use snapbox::cmd::Command;
use snapbox::{assert_data_eq, file};

const HOD: &str = env!("CARGO_BIN_EXE_hod");
const NEWEST: &str = "9.9.9 3f2b1c9d8e7f6a5b4c3d2e1f";

fn hod() -> Command {
    Command::new(HOD).env("HOD_NO_UPDATE_CHECK", "1")
}

#[test]
fn the_root_screen_lists_each_command() {
    let mut command = hodstack::command();
    let screen = command.render_help().to_string();
    assert_data_eq!(screen, file!["snapshots/root.txt"]);
}

#[test]
fn the_root_screen_names_the_path_of_this_binary() {
    let mut command = hodstack::command();
    let binary = std::env::current_exe().expect("a running test carries its own path");
    let screen = command.render_help().to_string();
    assert!(screen.trim_end().ends_with(&binary.display().to_string()));
}

#[test]
fn the_root_screen_carries_a_style() {
    let mut command = hodstack::command();
    let screen = command.render_help().ansi().to_string();
    assert!(screen.contains('\u{1b}'), "the screen carries no style");
}

#[test]
fn completions_prints_a_zsh_script() {
    let output = std::process::Command::new(HOD)
        .args(["completions", "zsh"])
        .output()
        .expect("hod runs");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("#compdef hod"));
}

#[test]
fn list_names_each_shipped_skill_even_without_a_project() {
    let dir = tempfile::tempdir().expect("a temp dir");

    let output = std::process::Command::new(HOD)
        .arg("list")
        .current_dir(dir.path())
        .output()
        .expect("hod runs");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("USER SKILLS"));
    assert!(stdout.contains("init"));
    assert!(stdout.contains("deps-upgrade"));
}

#[test]
fn a_skill_without_a_project_is_a_fault() {
    let dir = tempfile::tempdir().expect("a temp dir");

    hod()
        .arg("deps-upgrade")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr_eq("error: no `.hod` directory found here; run `hod init` first\n");
}

#[test]
fn a_skill_that_is_absent_is_a_fault() {
    let dir = tempfile::tempdir().expect("a temp dir");
    fs::create_dir(dir.path().join(".hod")).expect("a fresh .hod");

    hod()
        .arg("nope")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr_eq(
            "error: no skill named `nope`; check `.hod/skills/` or run `hod init` to see the shipped skills\n",
        );
}

#[test]
fn init_keeps_a_file_that_exists_and_writes_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    fs::write(dir.path().join("AGENTS.md"), "mine").expect("a file of the user");

    hod().arg("init").current_dir(dir.path()).assert().failure();

    assert!(!dir.path().join(".hod").exists());
    assert_eq!(
        fs::read_to_string(dir.path().join("AGENTS.md")).expect("the file"),
        "mine"
    );
}

#[cfg(unix)]
fn agent(dir: &Path, binary: &str, record: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt as _;

    let bin = dir.join("bin");
    fs::create_dir_all(&bin).expect("a bin dir");
    let file = bin.join(binary);
    fs::write(
        &file,
        format!("#!/bin/sh\nprintf '%s' \"$*\" > {}\n", record.display()),
    )
    .expect("a fake agent");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o755))
        .expect("an executable fake agent");
    bin
}

#[cfg(unix)]
#[test]
fn init_hands_off_to_the_init_skill_then_a_later_skill_starts_its_own() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let record = dir.path().join("record");
    let bin = agent(dir.path(), "claude", &record);

    hod()
        .arg("init")
        .current_dir(dir.path())
        .env("PATH", &bin)
        .env_remove("HOD_AGENT")
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&record).expect("the record"), "/init");

    fs::write(&record, "").expect("a cleared record");

    hod()
        .arg("deps-upgrade")
        .current_dir(dir.path())
        .env("PATH", &bin)
        .env_remove("HOD_AGENT")
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&record).expect("the record"),
        "/deps-upgrade"
    );
}

fn target() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "aarch64") => "aarch64-unknown-linux-musl",
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        (os, arch) => panic!("no build for {os} {arch}"),
    }
}

fn release(dir: &Path, body: &str, sound: bool) -> String {
    let name = if cfg!(windows) { "hod.exe" } else { "hod" };
    let file = format!("hod-{}.tar.gz", target());
    let staging = dir.join("staging");
    fs::create_dir_all(&staging).expect("a staging dir");
    fs::write(staging.join(name), body).expect("a staged binary");

    let status = std::process::Command::new("tar")
        .arg("-czf")
        .arg(dir.join(&file))
        .arg("-C")
        .arg(&staging)
        .arg(name)
        .status()
        .expect("tar runs");
    assert!(status.success());

    let sum = Sha256::digest(fs::read(dir.join(&file)).expect("the archive"))
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        });
    let sum = if sound { sum } else { "0".repeat(64) };

    fs::write(dir.join("checksums.txt"), format!("{sum}  {file}\n")).expect("checksums.txt");
    fs::write(dir.join("version.txt"), format!("{NEWEST}\n")).expect("version.txt");

    format!("file://{}", dir.display())
}

fn installed(home: &Path) -> PathBuf {
    let binary = home.join(if cfg!(windows) { "hod.exe" } else { "hod" });
    fs::copy(HOD, &binary).expect("a copy of this build");
    binary
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
    let home = tempfile::tempdir().expect("a home dir");
    let dir = tempfile::tempdir().expect("a release dir");
    let address = release(dir.path(), "the newest build", false);
    let binary = installed(home.path());
    let before = fs::read(&binary).expect("the installed binary");

    update(&binary, home.path(), &address).assert().failure().stderr_eq(format!(
        "error: cannot install 9.9.9 (3f2b1c9): the checksum of hod-{}.tar.gz does not agree with checksums.txt\n",
        target()
    ));

    assert_eq!(fs::read(&binary).expect("the installed binary"), before);
}

#[test]
fn update_installs_the_newest_build_when_the_checksum_agrees() {
    let home = tempfile::tempdir().expect("a home dir");
    let dir = tempfile::tempdir().expect("a release dir");
    let address = release(dir.path(), "the newest build", true);
    let binary = installed(home.path());

    update(&binary, home.path(), &address).assert().success();

    assert_eq!(
        fs::read_to_string(&binary).expect("the installed binary"),
        "the newest build"
    );
}
