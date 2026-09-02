---
name: init
description: "Learn the intention of this project and write it into `.hod/PROJECT.md`. Use right after `hod init` writes the placeholder file."
disable-model-invocation: true
---

# Init

Replace the placeholder text of `.hod/PROJECT.md` with the real intention of this project.

## 1. Read the project

Read `.hod/PROJECT.md`. It holds the placeholder that `hod init` wrote.

Read the manifest of the project, such as `package.json`, `composer.json`, `Cargo.toml`, `pyproject.toml`, or `go.mod`. Read `README.md` when it exists. Find the command that installs the dependencies, the command that runs the tests, and the command that starts the program.

List each top-level directory of the project.

## 2. Ask the user

Ask the user one question at a time.

Ask what the project does and who uses it, when the manifest and the README leave that answer unclear. Ask for a command that section 1 did not find with certainty. Ask the user to name the function of a directory that section 1 could not describe from its content alone.

Ask no question whose answer section 1 already found with certainty.

## 3. Write `.hod/PROJECT.md`

Write the intention of the project, the three commands, and the directory list into `.hod/PROJECT.md`. Keep the file short. Write no history of this task and no sentence about `hod` itself.

## 4. Stop

Report the file that you wrote. Ask the user no other question after this task.
