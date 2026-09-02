use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};

use crate::front::Front;
use crate::skills::{self, Skill};

pub const AGENTS: &str = "AGENTS.md";
pub const CLAUDE: &str = "CLAUDE.md";
pub const INTENTION: &str = ".hod/PROJECT.md";
pub const SKILLS_CLIENT: &str = ".claude/skills";

pub const RULES: &str = include_str!("../templates/rules.md");
pub const SEED: &str = include_str!("../templates/PROJECT.md");
pub const IMPORT: &str = "@AGENTS.md\n";

#[derive(Debug)]
pub struct Rule {
    pub file: String,
    pub front: Front,
}

pub struct Project {
    root: PathBuf,
}

impl Project {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    pub fn lock_path(&self) -> PathBuf {
        self.path(".hod/lock")
    }

    pub fn exists(&self) -> bool {
        self.path(".hod").is_dir()
    }

    pub fn rules(&self) -> Result<Vec<Rule>> {
        let dir = self.path(".hod/rules");
        let Ok(entries) = fs::read_dir(&dir) else {
            return Ok(Vec::new());
        };

        let mut rules = Vec::new();

        for entry in entries {
            let path = entry
                .with_context(|| format!("cannot read `{}`", dir.display()))?
                .path();

            if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
                continue;
            }

            let file = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let stem = file.trim_end_matches(".md").to_owned();
            let text = fs::read_to_string(&path)
                .with_context(|| format!("cannot read `{}`", path.display()))?;

            rules.push(Rule {
                file,
                front: Front::read(&text, &stem),
            });
        }

        rules.sort_by(|one, other| one.file.cmp(&other.file));

        Ok(rules)
    }

    pub fn installed_skills(&self) -> Result<Vec<Skill>> {
        skills::installed(&self.path(".hod/skills"))
    }

    pub fn skills(&self) -> Result<Vec<Skill>> {
        skills::local(&self.path(".hod/skills"))
    }
}

pub fn agents_md(rules: &[Rule]) -> String {
    let mut text = RULES.to_owned();

    if rules.is_empty() {
        return text;
    }

    text.push_str("\n---\n\n## 6. The rules of this project\n\n");
    text.push_str("Read the file of a rule when its subject reaches your task.\n\n");

    for rule in rules {
        text.push_str(&row(rule));
    }

    text
}

fn row(rule: &Rule) -> String {
    let path = format!(".hod/rules/{}", rule.file);

    if rule.front.description.is_empty() {
        return format!("- [{}]({path})\n", rule.front.name);
    }

    format!(
        "- [{}]({path}): {}\n",
        rule.front.name, rule.front.description
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(file: &str, name: &str, description: &str) -> Rule {
        Rule {
            file: file.to_owned(),
            front: Front {
                name: name.to_owned(),
                description: description.to_owned(),
                user: false,
            },
        }
    }

    #[test]
    fn the_agents_file_without_a_rule_is_the_template() {
        assert_eq!(agents_md(&[]), RULES);
    }

    #[test]
    fn each_rule_gives_one_row_with_its_path() {
        let text = agents_md(&[
            rule("no-yalc.md", "no-yalc", "Never use npm link"),
            rule("no-description.md", "no-description", ""),
        ]);

        assert!(text.starts_with(RULES));
        assert!(text.contains("## 6. The rules of this project"));
        assert!(text.contains("- [no-yalc](.hod/rules/no-yalc.md): Never use npm link\n"));
        assert!(text.contains("- [no-description](.hod/rules/no-description.md)\n"));
    }

    #[test]
    fn a_project_reads_each_rule_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let rules = dir.path().join(".hod/rules");
        fs::create_dir_all(&rules).unwrap();
        fs::write(rules.join("two.md"), "---\nname: two\n---\n").unwrap();
        fs::write(rules.join("one.md"), "---\nname: one\n---\n").unwrap();
        fs::write(rules.join("notes.txt"), "not a rule").unwrap();

        let read = Project::new(dir.path()).rules().unwrap();

        assert_eq!(read.len(), 2);
        assert_eq!(read[0].front.name, "one");
        assert_eq!(read[1].front.name, "two");
    }

    #[test]
    fn a_project_without_a_rules_directory_has_no_rule() {
        let dir = tempfile::tempdir().unwrap();

        assert!(Project::new(dir.path()).rules().unwrap().is_empty());
    }
}
