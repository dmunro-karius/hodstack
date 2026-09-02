# Context

A glossary of the terms this codebase uses. One word, one meaning. When a
near-synonym is tempting, this file says why to use the term instead.

## Terms

### coding agent

A binary on `PATH` that `hod` hands a skill prompt to: `claude`, `codex`,
`cursor-agent`, `opencode`, or `gemini` (`src/agent.rs`). `hod` detects the
first one found, or reads `HOD_AGENT` to force a choice.

_Avoid_: "agent" alone (too easily confused with **skill**, which is what
gets handed *to* a coding agent, not the program running it), "assistant",
"LLM", "model".

### skill

A directory containing a `SKILL.md` (YAML front matter with `name` and
`description`, plus body text) and optionally other files (`src/skills.rs`).
A skill is what a coding agent runs when a user types `/<name>`.

_Avoid_: "command", "prompt" (a skill is a file on disk, not the text a user
types).

### shipped skill

A skill baked into the `hod` binary at compile time via `include_str!`
(`skills::shipped()` — currently `init` and `deps-upgrade`). Exists even in
a project with no `.hod` directory yet.

### local skill

A skill read from disk at runtime, from a project's `.hod/skills/`
directory (`skills::local()`). A local skill with the same name as a
shipped skill overrides it.

_Avoid_: "custom skill", "user skill" — "local" is the word this codebase
uses; it means "read from this project's tree," not "written by a person"
(a local skill could itself be generated).

### installed skill

The result of merging shipped skills with local skills, local winning ties
(`skills::installed()`). This is the list a project actually has available
to run. `hod list` and `hod run_skill` both work off this list, never off
`shipped()` or `local()` alone.

### skills tree

The `.claude/skills/` directory (`project::SKILLS_CLIENT`) that `sync`
writes installed-skill files into. This is the path a coding agent like
Claude Code actually reads skills from — the project's own `.hod/skills/`
is `hod`'s source of truth, not the agent's.

### project

A wrapper (`project::Project`) around a root directory, giving typed access
to the paths `hod` cares about inside it: `.hod/PROJECT.md`, `.hod/rules/`,
`.hod/lock`, `.claude/skills/`.

_Avoid_: "repo" — a project is any directory `hod init` has run in; it need
not be a git repository.

### intention

The project's own seed file, `.hod/PROJECT.md` (`project::INTENTION`),
written once by `hod init` from a template (`project::SEED`) and then left
alone — `hod` never overwrites it after creation.

### rule

A Markdown file under `.hod/rules/`, with front matter, that `sync` folds
into `AGENTS.md`/`CLAUDE.md` (`project::Rule`).

### front matter

The YAML block at the top of a `SKILL.md` or rule file, parsed into a
`Front` (`src/front.rs`) with a `name` and a `description`.

### lock

The checksum ledger at `.hod/lock` (`lock::Lock`), one SHA-256 sum per file
`hod` has written. It exists so `sync` can tell a file it wrote itself from
a file a person edited or created by hand.

### ownership

`lock::Owner`, computed by comparing a file on disk against the lock:
**Absent** (file does not exist), **Ours** (file matches the lock's
checksum for it — `hod` wrote it and it is unchanged), **Theirs** (file
exists but does not match — a person changed it, or `hod` never wrote it).
`sync` skips a `Theirs` file rather than overwrite it, unless run with
`Mode::Force`.

### sync

The operation (`sync::sync()`) that writes `AGENTS.md`, `CLAUDE.md`, and
every installed skill into the skills tree, gated by ownership. Runs as
part of both `hod init` and `hod update`.

### mode

`sync::Mode`, one of three: **Check** (report what would change, write
nothing), **Write** (write files that are `Absent` or `Ours`, skip
`Theirs`), **Force** (write everything, including over `Theirs`).

### build

A release artifact described by `Build` (`src/update.rs`): a version plus
the full commit SHA it was built from, read from a release's
`version.txt`.

### `HOD_COMMIT`

A compile-time environment variable (`option_env!`), set only by the
release workflow's `cargo build` step, never by a developer's own `cargo
build`/`cargo run`. Embedded into the binary so `hod --version` can show
the commit and `hod update --check` can tell "this binary" from "the
newest release" without a network round trip doing more than a version
comparison.

_Avoid_: expecting a local dev build to carry this — see
`docs/13-hod-commit-and-ci.md`. A local build reporting itself as
perpetually out of date is expected, not a bug.

### `HOD_AGENT`

A runtime environment variable that forces `agent::find()` to a specific
coding agent binary, bypassing `PATH` detection.

### `HOD_RELEASE_URL`

A runtime environment variable overriding where `hod update` fetches
`version.txt`/`checksums.txt`/archives from. Used in this repo's own
manual verification to point at a local fixture directory instead of a
real GitHub release.

## Flagged ambiguities

- **"agent"** is not used for the coding agent's underlying LLM anywhere in
  this codebase — there is no separate "model" concept. If one is ever
  added (e.g. picking a model within a coding agent), give it its own term
  here rather than overloading **coding agent**.
- **"local"** (as in **local skill**) describes *where a skill is read
  from* (this project's `.hod/skills/`), not *who wrote it*. Don't read
  "local" as "user-authored."
