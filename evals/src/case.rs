use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Case {
    pub base: String,
    pub intent: String,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default = "default_threshold")]
    pub threshold: f64,
    #[serde(default, rename = "graders")]
    pub graders: Vec<Grader>,
}

fn default_threshold() -> f64 {
    1.0
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Match {
    #[default]
    Contains,
    NotContains,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Grader {
    ToolUsed {
        tool: String,
        #[serde(default)]
        input_match: Option<String>,
    },
    ToolOrder {
        tools: Vec<String>,
    },
    Regex {
        path: String,
        pattern: String,
        #[serde(default, rename = "match")]
        match_kind: Match,
    },
    FileContent {
        path: String,
        pattern: String,
        #[serde(default, rename = "match")]
        match_kind: Match,
    },
    FileExists {
        path: String,
    },
    GitClean,
    GitDirty,
    HeadUnmoved,
}

pub struct TestFile {
    pub case: Case,
    pub expectation: String,
}

pub fn read(path: &Path) -> Result<TestFile> {
    let text = fs::read_to_string(path).with_context(|| format!("cannot read `{}`", path.display()))?;

    let rest = text
        .strip_prefix("+++\n")
        .with_context(|| format!("`{}` must open with +++", path.display()))?;

    let Some((front, body)) = rest.split_once("\n+++") else {
        bail!("`{}` must close its front matter with +++", path.display())
    };

    let case: Case = toml::from_str(front)
        .with_context(|| format!("cannot read the front matter of `{}`", path.display()))?;

    Ok(TestFile {
        case,
        expectation: body.trim_start_matches('\n').to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_case_and_its_expectation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.md");
        fs::write(
            &path,
            concat!(
                "+++\n",
                "base = \"fake-php\"\n",
                "intent = \"do the thing\"\n",
                "\n",
                "[[graders]]\n",
                "type = \"file_exists\"\n",
                "path = \"composer.json\"\n",
                "+++\n",
                "\n",
                "The agent does the thing.\n",
            ),
        )
        .unwrap();

        let test = read(&path).unwrap();

        assert_eq!(test.case.base, "fake-php");
        assert_eq!(test.case.threshold, 1.0);
        assert_eq!(test.case.graders.len(), 1);
        assert_eq!(test.expectation, "The agent does the thing.\n");
    }

    #[test]
    fn a_file_without_closing_marks_is_a_fault() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.md");
        fs::write(&path, "+++\nbase = \"x\"\n").unwrap();

        assert!(read(&path).is_err());
    }
}
