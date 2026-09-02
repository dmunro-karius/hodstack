use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context as _, Result, bail};

use crate::base;
use crate::case::{self, Grader};
use crate::grade::{self, Verdict};

const RESERVED: [&str; 3] = ["src", "bases", "target"];

pub struct CaseRef {
    pub skill: String,
    pub name: String,
    pub dir: PathBuf,
}

pub fn discover(root: &Path) -> Result<Vec<CaseRef>> {
    let mut cases = Vec::new();

    for entry in fs::read_dir(root).with_context(|| format!("cannot read `{}`", root.display()))? {
        let entry = entry.with_context(|| format!("cannot read `{}`", root.display()))?;
        let skill_dir = entry.path();
        let skill = entry.file_name().to_string_lossy().into_owned();

        if !skill_dir.is_dir() || RESERVED.contains(&skill.as_str()) || skill.starts_with('.') {
            continue;
        }

        for case_entry in
            fs::read_dir(&skill_dir).with_context(|| format!("cannot read `{}`", skill_dir.display()))?
        {
            let case_entry = case_entry.with_context(|| format!("cannot read `{}`", skill_dir.display()))?;
            let case_dir = case_entry.path();

            if !case_dir.join("test.md").is_file() {
                continue;
            }

            cases.push(CaseRef {
                skill: skill.clone(),
                name: case_entry.file_name().to_string_lossy().into_owned(),
                dir: case_dir,
            });
        }
    }

    cases.sort_by(|one, other| (&one.skill, &one.name).cmp(&(&other.skill, &other.name)));

    Ok(cases)
}

fn matches(case: &CaseRef, filter: Option<&str>) -> bool {
    let Some(filter) = filter else { return true };
    let full = format!("{}/{}", case.skill, case.name);
    case.skill == filter || full == filter
}

pub fn main(list: bool, filter: Option<&str>, runs: usize) -> Result<ExitCode> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cases: Vec<_> = discover(root)?.into_iter().filter(|case| matches(case, filter)).collect();

    if cases.is_empty() {
        bail!("no case matches `{}`", filter.unwrap_or(""));
    }

    if list {
        for case in &cases {
            println!("{}/{}", case.skill, case.name);
        }
        return Ok(ExitCode::SUCCESS);
    }

    let mut failures = 0;

    for case in &cases {
        let outcome = run_case(root, case, runs)?;
        report(case, &outcome);

        if !outcome.pass {
            failures += 1;
        }
    }

    println!();
    println!("{} case(s), {failures} failing", cases.len());

    Ok(if failures == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE })
}

struct Outcome {
    pass: bool,
    passed: usize,
    runs: usize,
    threshold: f64,
    expectation: String,
    graders: Vec<(String, Verdict)>,
}

fn run_case(root: &Path, entry: &CaseRef, runs: usize) -> Result<Outcome> {
    let test = case::read(&entry.dir.join("test.md"))?;
    let cached = base::seed(root, &test.case.base)?;

    let mut passed = 0;
    let mut last = Vec::new();

    for _ in 0..runs {
        let work = env::temp_dir().join(format!(
            "hod-eval-{}-{}-{}-{}",
            entry.skill,
            entry.name,
            std::process::id(),
            passed + last.len()
        ));
        let _ = fs::remove_dir_all(&work);

        let attempt = run_case_in(&work, &entry.dir, &cached, &test.case, &test.case.graders);
        let _ = fs::remove_dir_all(&work);

        let results = attempt?;

        if results.iter().all(|(_, verdict)| verdict.pass) {
            passed += 1;
        }

        last = results;
    }

    let rate = f64::from(u32::try_from(passed).unwrap_or(u32::MAX)) / f64::from(u32::try_from(runs).unwrap_or(1));

    Ok(Outcome {
        pass: rate >= test.case.threshold,
        passed,
        runs,
        threshold: test.case.threshold,
        expectation: test.expectation,
        graders: last,
    })
}

fn run_case_in(
    work: &Path,
    case_dir: &Path,
    cached: &Path,
    case: &case::Case,
    graders: &[Grader],
) -> Result<Vec<(String, Verdict)>> {
    base::copy_into(cached, work)?;

    git(work, &["init", "-q"])?;
    git(work, &["add", "-A"])?;
    git(work, &["commit", "-q", "-m", "base"])?;

    let setup = case_dir.join("setup.sh");
    if setup.is_file() {
        run_script(&setup, work)?;
    }

    let head_before = head(work)?;

    let trace_file = work.join(".eval-trace.jsonl");
    run_agent(work, &trace_file, case)?;

    let events = grade::trace(&trace_file);

    Ok(graders
        .iter()
        .map(|grader| (label(grader), grade::grade(grader, work, &events, &head_before)))
        .collect())
}

fn label(grader: &Grader) -> String {
    match grader {
        Grader::ToolUsed { tool, .. } => format!("tool_used({tool})"),
        Grader::ToolOrder { tools } => format!("tool_order({})", tools.join(",")),
        Grader::Regex { path, .. } => format!("regex({path})"),
        Grader::FileContent { path, .. } => format!("file_content({path})"),
        Grader::FileExists { path } => format!("file_exists({path})"),
        Grader::GitClean => "git_clean".to_owned(),
        Grader::GitDirty => "git_dirty".to_owned(),
        Grader::HeadUnmoved => "head_unmoved".to_owned(),
    }
}

fn run_script(script: &Path, work: &Path) -> Result<()> {
    let status = Command::new("sh")
        .arg(script)
        .current_dir(work)
        .status()
        .with_context(|| format!("cannot run `{}`", script.display()))?;

    if !status.success() {
        bail!("`{}` exits with {status}", script.display());
    }

    Ok(())
}

fn run_agent(work: &Path, trace_file: &Path, case: &case::Case) -> Result<()> {
    let Some(command) = env::var_os("HOD_EVAL_AGENT_CMD") else {
        bail!("no agent command configured; set HOD_EVAL_AGENT_CMD")
    };

    let status = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .current_dir(work)
        .env("HOD_EVAL_TRACE_FILE", trace_file)
        .env("HOD_EVAL_INTENT", &case.intent)
        .env("HOD_EVAL_ALLOWED_TOOLS", case.allowed_tools.join(","))
        .status()
        .with_context(|| format!("cannot run `{}`", OsStr::new(&command).to_string_lossy()))?;

    if !status.success() {
        bail!("the agent command exits with {status}");
    }

    Ok(())
}

fn git(work: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .args(args)
        .current_dir(work)
        .env("GIT_AUTHOR_NAME", "hod-evals")
        .env("GIT_AUTHOR_EMAIL", "hod-evals@example.com")
        .env("GIT_COMMITTER_NAME", "hod-evals")
        .env("GIT_COMMITTER_EMAIL", "hod-evals@example.com")
        .status()
        .context("cannot run `git`")?;

    if !status.success() {
        bail!("`git {}` exits with {status}", args.join(" "));
    }

    Ok(())
}

fn head(work: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(work)
        .output()
        .context("cannot run `git rev-parse HEAD`")?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn report(case: &CaseRef, outcome: &Outcome) {
    let word = if outcome.pass { "pass" } else { "fail" };

    if outcome.runs > 1 {
        println!(
            "{word}  {}/{} ({}/{} runs, wants {})",
            case.skill, case.name, outcome.passed, outcome.runs, outcome.threshold
        );
    } else {
        println!("{word}  {}/{}", case.skill, case.name);
    }

    for (label, verdict) in &outcome.graders {
        if !verdict.pass {
            println!("       {label}: {}", verdict.detail);
        }
    }

    if !outcome.pass {
        println!("       expects: {}", outcome.expectation.trim());
    }
}
