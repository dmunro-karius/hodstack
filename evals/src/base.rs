use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};

pub fn seed(root: &Path, name: &str) -> Result<PathBuf> {
    let cache = root.join("target").join("eval-cache").join(name);

    if cache.is_dir() {
        return Ok(cache);
    }

    let script = root.join("bases").join(name).join("seed.sh");

    if !script.is_file() {
        bail!("no base named `{name}` (looked for `{}`)", script.display())
    }

    fs::create_dir_all(&cache).with_context(|| format!("cannot create `{}`", cache.display()))?;

    let status = Command::new("sh")
        .arg(&script)
        .current_dir(&cache)
        .status()
        .with_context(|| format!("cannot run `{}`", script.display()))?;

    if !status.success() {
        let _ = fs::remove_dir_all(&cache);
        bail!("`{}` exits with {status}", script.display());
    }

    Ok(cache)
}

pub fn copy_into(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).with_context(|| format!("cannot create `{}`", dst.display()))?;

    for entry in fs::read_dir(src).with_context(|| format!("cannot read `{}`", src.display()))? {
        let entry = entry.with_context(|| format!("cannot read `{}`", src.display()))?;
        let path = entry.path();
        let name = entry.file_name();

        if name == ".git" {
            continue;
        }

        let target = dst.join(&name);

        if path.is_dir() {
            copy_into(&path, &target)?;
        } else {
            fs::copy(&path, &target).with_context(|| format!("cannot copy `{}`", path.display()))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_into_skips_git_and_keeps_the_tree() {
        let src = tempfile::tempdir().unwrap();
        fs::create_dir_all(src.path().join(".git")).unwrap();
        fs::write(src.path().join(".git/x"), "nope").unwrap();
        fs::create_dir_all(src.path().join("src")).unwrap();
        fs::write(src.path().join("src/App.php"), "<?php\n").unwrap();

        let dst = tempfile::tempdir().unwrap();
        copy_into(src.path(), dst.path()).unwrap();

        assert!(!dst.path().join(".git").exists());
        assert_eq!(fs::read_to_string(dst.path().join("src/App.php")).unwrap(), "<?php\n");
    }

    #[test]
    fn seed_rejects_an_unknown_base() {
        let root = tempfile::tempdir().unwrap();
        assert!(seed(root.path(), "nope").is_err());
    }
}
