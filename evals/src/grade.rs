use std::fs;
use std::path::Path;
use std::process::Command;

use regex::Regex;
use serde::Deserialize;

use crate::case::{Grader, Match};

#[derive(Debug, Deserialize, Clone)]
pub struct Event {
    pub tool: String,
    #[serde(default)]
    pub input: String,
}

pub fn trace(path: &Path) -> Vec<Event> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };

    text.lines().filter_map(|line| serde_json::from_str(line).ok()).collect()
}

#[derive(Debug)]
pub struct Verdict {
    pub pass: bool,
    pub detail: String,
}

impl Verdict {
    fn pass() -> Self {
        Self { pass: true, detail: String::new() }
    }

    fn fail(detail: impl Into<String>) -> Self {
        Self { pass: false, detail: detail.into() }
    }
}

pub fn grade(grader: &Grader, work: &Path, trace: &[Event], head_before: &str) -> Verdict {
    match grader {
        Grader::ToolUsed { tool, input_match } => tool_used(trace, tool, input_match.as_deref()),
        Grader::ToolOrder { tools } => tool_order(trace, tools),
        Grader::Regex { path, pattern, match_kind } | Grader::FileContent { path, pattern, match_kind } => {
            file_pattern(work, path, pattern, *match_kind)
        }
        Grader::FileExists { path } => file_exists(work, path),
        Grader::GitClean => git_clean(work, true),
        Grader::GitDirty => git_clean(work, false),
        Grader::HeadUnmoved => head_unmoved(work, head_before),
    }
}

fn tool_used(trace: &[Event], tool: &str, input_match: Option<&str>) -> Verdict {
    let found = trace.iter().any(|event| {
        if event.tool != tool {
            return false;
        }

        let Some(pattern) = input_match else {
            return true;
        };

        Regex::new(pattern).is_ok_and(|regex| regex.is_match(&event.input))
    });

    if found {
        Verdict::pass()
    } else {
        Verdict::fail(format!("no recorded use of `{tool}` matches this grader"))
    }
}

fn tool_order(trace: &[Event], tools: &[String]) -> Verdict {
    let mut wanted = tools.iter();
    let mut next = wanted.next();

    for event in trace {
        if next == Some(&event.tool) {
            next = wanted.next();
        }
    }

    if next.is_none() {
        Verdict::pass()
    } else {
        Verdict::fail(format!("the trace never reaches `{}` in order", tools.join(" -> ")))
    }
}

fn file_pattern(work: &Path, path: &str, pattern: &str, match_kind: Match) -> Verdict {
    let full = work.join(path);

    let Ok(text) = fs::read_to_string(&full) else {
        return Verdict::fail(format!("cannot read `{path}`"));
    };

    let Ok(regex) = Regex::new(pattern) else {
        return Verdict::fail(format!("`{pattern}` is not a valid regex"));
    };

    let hit = regex.is_match(&text);

    match (match_kind, hit) {
        (Match::Contains, true) => Verdict::pass(),
        (Match::Contains, false) => Verdict::fail(format!("`{path}` does not match `{pattern}`")),
        (Match::NotContains, false) => Verdict::pass(),
        (Match::NotContains, true) => Verdict::fail(format!("`{path}` still matches `{pattern}`")),
    }
}

fn file_exists(work: &Path, path: &str) -> Verdict {
    if work.join(path).exists() {
        Verdict::pass()
    } else {
        Verdict::fail(format!("`{path}` does not exist"))
    }
}

fn git_clean(work: &Path, want_clean: bool) -> Verdict {
    let Ok(output) = Command::new("git").args(["status", "--porcelain"]).current_dir(work).output() else {
        return Verdict::fail("cannot run `git status`");
    };

    let clean = output.stdout.is_empty();

    match (clean, want_clean) {
        (true, true) | (false, false) => Verdict::pass(),
        (false, true) => Verdict::fail("the working tree is dirty"),
        (true, false) => Verdict::fail("the working tree is clean"),
    }
}

fn head_unmoved(work: &Path, head_before: &str) -> Verdict {
    let Ok(output) = Command::new("git").args(["rev-parse", "HEAD"]).current_dir(work).output() else {
        return Verdict::fail("cannot run `git rev-parse HEAD`");
    };

    let head_now = String::from_utf8_lossy(&output.stdout).trim().to_owned();

    if head_now == head_before {
        Verdict::pass()
    } else {
        Verdict::fail("HEAD moved during the run")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(tool: &str, input: &str) -> Event {
        Event { tool: tool.to_owned(), input: input.to_owned() }
    }

    #[test]
    fn tool_used_requires_the_input_pattern_when_one_is_given() {
        let trace = vec![event("Bash", "composer outdated"), event("Edit", "composer.json")];

        assert!(tool_used(&trace, "Bash", Some("outdated")).pass);
        assert!(!tool_used(&trace, "Bash", Some("nope")).pass);
        assert!(tool_used(&trace, "Edit", None).pass);
        assert!(!tool_used(&trace, "Write", None).pass);
    }

    #[test]
    fn tool_order_allows_gaps_but_not_reordering() {
        let trace = vec![event("Read", "a"), event("Bash", "b"), event("Edit", "c")];

        assert!(tool_order(&trace, &["Read".to_owned(), "Edit".to_owned()]).pass);
        assert!(!tool_order(&trace, &["Edit".to_owned(), "Read".to_owned()]).pass);
    }

    #[test]
    fn file_pattern_honors_not_contains() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("composer.json"), "1.0.0").unwrap();

        assert!(file_pattern(dir.path(), "composer.json", "1\\.0\\.0", Match::Contains).pass);
        assert!(!file_pattern(dir.path(), "composer.json", "1\\.0\\.0", Match::NotContains).pass);
        assert!(file_pattern(dir.path(), "composer.json", "9\\.9\\.9", Match::NotContains).pass);
    }
}
