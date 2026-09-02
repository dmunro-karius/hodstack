use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result};

use crate::front::Front;

const DEPS_UPGRADE: &str = include_str!("../skills/deps-upgrade/SKILL.md");
const INIT_SKILL: &str = include_str!("../skills/init/SKILL.md");

pub const INIT: &str = "init";

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub files: Vec<(String, String)>,
}

impl Skill {
    pub fn front(&self) -> Front {
        let text = self
            .files
            .iter()
            .find(|(file, _)| file == "SKILL.md")
            .map_or("", |(_, text)| text.as_str());

        Front::read(text, &self.name)
    }
}

pub fn shipped() -> Vec<Skill> {
    vec![
        Skill {
            name: INIT.to_owned(),
            files: vec![("SKILL.md".to_owned(), INIT_SKILL.to_owned())],
        },
        Skill {
            name: "deps-upgrade".to_owned(),
            files: vec![("SKILL.md".to_owned(), DEPS_UPGRADE.to_owned())],
        },
    ]
}

pub fn local(dir: &Path) -> Result<Vec<Skill>> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(Vec::new());
    };

    let mut skills = Vec::new();

    for entry in entries {
        let path = entry
            .with_context(|| format!("cannot read `{}`", dir.display()))?
            .path();

        if !path.join("SKILL.md").is_file() {
            continue;
        }

        skills.push(Skill {
            name: file_name(&path),
            files: walk(&path, &path)?,
        });
    }

    skills.sort_by(|one, other| one.name.cmp(&other.name));

    Ok(skills)
}

pub fn installed(project_skills_dir: &Path) -> Result<Vec<Skill>> {
    let mut skills = shipped();

    for mine in local(project_skills_dir)? {
        match skills.iter().position(|skill| skill.name == mine.name) {
            Some(at) => skills[at] = mine,
            None => skills.push(mine),
        }
    }

    Ok(skills)
}

fn walk(skill: &Path, dir: &Path) -> Result<Vec<(String, String)>> {
    let entries = fs::read_dir(dir).with_context(|| format!("cannot read `{}`", dir.display()))?;

    let mut files = Vec::new();

    for entry in entries {
        let path = entry
            .with_context(|| format!("cannot read `{}`", dir.display()))?
            .path();

        if file_name(&path).starts_with('.') {
            continue;
        }

        if path.is_dir() {
            files.extend(walk(skill, &path)?);
            continue;
        }

        let text = fs::read_to_string(&path)
            .with_context(|| format!("cannot read `{}`", path.display()))?;

        files.push((relative(skill, &path), text));
    }

    files.sort_by(|one, other| one.0.cmp(&other.0));

    Ok(files)
}

fn relative(skill: &Path, file: &Path) -> String {
    file.strip_prefix(skill)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn front_of(skill: &Skill) -> Front {
        skill.front()
    }

    #[test]
    fn the_shipped_skill_carries_a_name_that_agrees_with_its_front_matter() {
        for skill in shipped() {
            let front = front_of(&skill);
            assert_eq!(front.name, skill.name);
            assert!(!front.description.is_empty());
        }
    }

    #[test]
    fn a_directory_without_a_skill_gives_no_skill() {
        let dir = tempfile::tempdir().unwrap();

        assert!(local(dir.path()).unwrap().is_empty());
        assert!(local(&dir.path().join("nothing")).unwrap().is_empty());
    }

    #[test]
    fn a_local_skill_carries_each_file_and_its_front_matter() {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("deploy");
        fs::create_dir_all(skill.join("references")).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: deploy\ndescription: Deploy this project\n---\n",
        )
        .unwrap();
        fs::write(skill.join("references/hosts.md"), "one").unwrap();

        let skills = local(dir.path()).unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "deploy");
        assert_eq!(front_of(&skills[0]).description, "Deploy this project");
        assert_eq!(
            skills[0].files,
            vec![
                ("SKILL.md".to_owned(), skills[0].files[0].1.clone()),
                ("references/hosts.md".to_owned(), "one".to_owned()),
            ]
        );
    }

    #[test]
    fn a_local_skill_takes_the_name_of_a_shipped_skill() {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("deps-upgrade");
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: deps-upgrade\ndescription: Mine.\n---\n",
        )
        .unwrap();

        let found = installed(dir.path())
            .unwrap()
            .into_iter()
            .find(|skill| skill.name == "deps-upgrade")
            .unwrap();

        assert_eq!(front_of(&found).description, "Mine.");
    }
}
