use std::fs;
use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_evals");
const CASE: &str = "deps-upgrade/raises-the-minor";

#[test]
fn listing_names_the_sample_case() {
    let output = Command::new(BIN).arg("--list").output().unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains(CASE));
}

#[test]
fn a_case_with_no_agent_command_is_a_fault() {
    Command::new(BIN)
        .arg(CASE)
        .env_remove("HOD_EVAL_AGENT_CMD")
        .assert_fails_with("no agent command configured");
}

#[cfg(unix)]
#[test]
fn the_sample_case_passes_against_an_agent_that_does_the_task() {
    let dir = tempfile::tempdir().unwrap();
    let script = fake_agent(dir.path(), true);

    let output = Command::new(BIN)
        .arg(CASE)
        .env("HOD_EVAL_AGENT_CMD", &script)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stdout));
}

#[cfg(unix)]
#[test]
fn the_sample_case_fails_against_an_agent_that_does_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let script = fake_agent(dir.path(), false);

    let output = Command::new(BIN)
        .arg(CASE)
        .env("HOD_EVAL_AGENT_CMD", &script)
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[cfg(unix)]
fn fake_agent(dir: &Path, does_the_task: bool) -> String {
    use std::os::unix::fs::PermissionsExt as _;

    let script = dir.join("agent.sh");
    let body = if does_the_task {
        concat!(
            "#!/bin/sh\n",
            "cat > composer.json <<'EOF'\n",
            "{\n",
            "    \"require\": {\n",
            "        \"acme/example\": \"1.2.0\"\n",
            "    }\n",
            "}\n",
            "EOF\n",
            "printf '{\"tool\":\"Edit\",\"input\":\"composer.json\"}\\n' >> \"$HOD_EVAL_TRACE_FILE\"\n",
        )
    } else {
        "#!/bin/sh\ntrue\n"
    };

    fs::write(&script, body).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    script.display().to_string()
}

trait AssertFails {
    fn assert_fails_with(&mut self, needle: &str);
}

impl AssertFails for Command {
    fn assert_fails_with(&mut self, needle: &str) {
        let output = self.output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(needle));
    }
}
