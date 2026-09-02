# Hodstack

Hodstack makes coding agents more productive.

## Installation

On macOS and Linux:

```sh
curl -fsSL https://github.com/dmunro-karius/hodstack/releases/latest/download/install.sh | sh
```

On Windows:

```powershell
irm https://github.com/dmunro-karius/hodstack/releases/latest/download/install.ps1 | iex
```

From source:

```sh
cargo install --path .
```

Puts `hod` on your `PATH` (requires `~/.cargo/bin` on `PATH`).

## Usage

```sh
hod init
```

Writes `AGENTS.md`, `CLAUDE.md`, and installs every skill into
`.claude/skills/`, so Claude Code can find and run them as slash commands.
Safe to rerun — it's how you fold in new rules and skills too.

Refuses to write anything if `AGENTS.md` or `CLAUDE.md` already exists.
Move its text into `.hod/PROJECT.md` and run `hod init` again. On success,
it opens your coding agent into the `init` skill, which asks about this
project and fills in `.hod/PROJECT.md` for you.

```
AGENTS.md                        hod writes this
CLAUDE.md                        hod writes this (imports AGENTS.md for Claude Code)
.hod/lock                        hod writes this
.hod/PROJECT.md                  what this project is
.hod/rules/*.md                  one rule per file, linked from AGENTS.md
.hod/skills/<name>/SKILL.md      your own skills
.claude/skills/<name>/SKILL.md   installed copy hod writes; gitignored
```

Tell your agent when it gets something wrong. It writes the rule in
`.hod/rules/`, then run `hod init` again to link it from `AGENTS.md`.

```sh
hod deps-upgrade
```

Runs the `deps-upgrade` skill: sends `/deps-upgrade` to `claude`, which
picks it up from `.claude/skills/deps-upgrade/` (installed there by
`hod init`). Works from any subdirectory of the project.

`hod init` never overwrites a file you've hand-edited. It hashes every file
it writes into `.hod/lock`; if the file on disk no longer matches that hash,
it's yours, and `hod init` skips it and exits non-zero. Rerun with
`hod init --force` to write over it anyway.

## Status

MVP. One shipped skill (`deps-upgrade`), one target agent (Claude Code —
skills install into `.claude/skills/` only, not other clients). `AGENTS.md`'s
style-guide header, `PROJECT.md`'s seed template, and the `deps-upgrade`
skill are adapted from [hodstack/hodstack](https://github.com/hodstack/hodstack)
(MIT — see `NOTICE.md`).
