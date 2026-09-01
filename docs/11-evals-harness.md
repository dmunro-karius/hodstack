# Evals harness: regression-test skill *text*, not just code

## What's missing

There is no way to verify that a change to a skill's `SKILL.md` (prose
instructions read by a model) doesn't silently break the model's behavior.
Unit tests can verify the `hod` binary's code; they can't verify that the
`deps-upgrade` skill still tells a coding agent to check for a dirty git
tree before running.

## What the remote repo does

`evals/` is a **separate Cargo workspace** (deliberately not a member of
`cli/`'s workspace — see the "why" below) that runs a real coding agent
against a real seeded project and grades the transcript + resulting files.

### Directory shape (`evals/AGENTS.md`, section 1)

```
<skill>/<case>/
├── setup.sh      plant the precondition of this case
├── test.md       the graders in the front matter, the expectation in the body
└── fixtures/     the files of this case, written over the base
```

### A real example case

`evals/deps-upgrade/raises-the-minor-in-a-real-project/test.md`:

```
+++
base = "laravel"
intent = "Upgrade the dependencies of this project to newer versions and keep the tests green."
allowed_tools = ["Bash", "Read", "Write", "Edit", "Grep", "Glob", "WebFetch"]

[[graders]]
type = "tool_used"
tool = "Bash"
input_match = "composer outdated --direct"

[[graders]]
type = "file_content"
path = "composer.json"
pattern = "9\\.16\\.0"
match = "not_contains"

[[graders]]
type = "git_dirty"
+++

The agent reports that it raised `league/csv` from `9.16.0` to a newer minor
version, and it gives both versions. It reports that it ran the tests of
this project after that change and that those tests passed. It asked the
user no question before it raised `league/csv`. This expectation holds
whatever the agent did with a package that has a newer major version:
raising it, keeping it, or asking the user about it all meet this
expectation.
```

`evals/deps-upgrade/raises-the-minor-in-a-real-project/setup.sh`:

```sh
set -eu

composer require "league/csv:9.16.0" --no-interaction --no-progress --no-audit --quiet

git add -A
git commit --quiet -m "the case"
```

### Two kinds of assertion (`evals/AGENTS.md`, section 2)

- **Graders** (TOML front matter) — mechanical, no model involved:
  `tool_used`, `tool_order`, `regex`, `file_content`, `file_exists`,
  `git_clean`, `git_dirty`, `head_unmoved` (implemented in `evals/src/
  case.rs`). A run must pass *every* grader.
- **Expectation** (the Markdown body) — a separate judge model reads the
  agent's own trace/report and weighs whether the expectation holds. Rule
  from the AGENTS.md: **write the expectation for what the report of the
  agent shows**, not for a raw fact of the filesystem — "the report doesn't
  mention it" fails even if the file itself is technically correct, because
  the judge only reads the report and the commands, never the project
  directly.

`threshold` (default `1.0`) sets the pass rate a case needs across
`--runs` repetitions — used to measure flaky/probabilistic cases (e.g.
`0.67` for 2-of-3), not as a way to lower the bar for a case that should
just pass reliably.

### Bases (`evals/bases/<name>/seed.sh`, section 4)

A base is a real seeded project (`catalogue`, `laravel` here), built once
into `.cache/<name>` and then handed to each case as a **copy-on-write
clone** (cheap, <1s, no disk cost) rather than re-seeded per case.

- `catalogue` is a synthetic base whose `packages/` directory contains
  every version of every dependency plus its own `CHANGELOG.md` release
  notes — so the set of "available upgrades" is a fact of the base, not of
  a live registry. No network call, deterministic, fast. This is the
  default choice, which is why `allowed_tools` defaults to excluding
  `WebFetch`.
- `laravel` is a real framework project hitting the real Composer
  registry — kept to **exactly one case**, deliberately, because a real
  registry drifts over time (a case can start failing purely because
  upstream shipped a new release, with the skill itself still correct).
  When a case needs "prove this works against a real ecosystem", pin one
  dependency backward in `setup.sh` to guarantee there's an upgrade to
  find, and pick a package the framework itself doesn't constrain (so
  the dependency resolver doesn't fight you).

Two other precision rules from `evals/AGENTS.md` worth keeping verbatim:

- `seed.sh` writes no `.git` — the harness does `git init`, commits the
  base, *then* runs `setup.sh`, so a `setup.sh` that wants a real diff in
  the working tree commits its own precondition change explicitly rather
  than the case starting already-dirty by accident.
- The harness's own files (`.eval/`, `.claude/`) must be hidden via
  `.git/info/exclude`, not `.gitignore` — otherwise they read as changes
  in the working tree and every `deps-upgrade` case fails at "check `git
  status`" before the agent does anything real. This is the kind of
  self-inflicted-failure trap worth a code comment/test if reimplementing.

### Running it (section 5)

```sh
cargo run --release -- --list          # name every case, run nothing
cargo run --release --                 # run every case of every skill
cargo run --release -- deps-upgrade    # run one skill
cargo run --release -- deps-upgrade/stops-on-a-dirty-tree  # run one case
```

- Stops at the **first failing run** by default and writes a diagnosis file
  to `.runs/diagnosis/` — meant to be handed directly to an agent tasked
  with fixing the skill, not summarized first. `--no-bail` runs the whole
  suite instead.
- `--jobs` (default 8) parallelizes across *skills*, not cases within a
  skill — cases of one skill run sequentially against one project,
  resetting via a fresh copy-on-write clone between cases (not `git
  reset`, since `vendor/`-style untracked directories wouldn't be cleaned
  by a git reset and would silently poison the next case).
- Cases sharing an identical precondition (same `setup.sh` + `fixtures/` +
  arm) are grouped so the precondition only runs once for the group.

### Why `evals/` is its own workspace

From `evals/AGENTS.md`, section 5:

> This crate holds its own `[workspace]`. Do not make it a member of the
> workspace of `cli/`: `cargo vet`, `cargo deny`, `cargo machete` and the
> MSRV check of `hod` then read the dependencies of this crate, and `cargo
> package` writes them into the crate that a user installs.

I.e. the evals harness's own dependencies (whatever it uses to drive an
agent CLI, parse TOML, call a judge model, etc.) must never leak into the
`hod` binary that ships to users, or into the supply-chain/MSRV checks that
gate the `hod` crate — see `09-build-quality-lints.md` and
`10-testing-infrastructure.md`.

## Implementation notes for this repo

- This is the largest, most independent piece of infrastructure in this
  doc set — it doesn't block or get blocked by docs 1–10, since it tests
  *skill text* (`skills/deps-upgrade/SKILL.md`), not the `hod` binary.
- Needs its own `Cargo.toml`/`Cargo.lock` at `evals/`, entirely separate
  from this repo's root `Cargo.toml`.
- Full source reference for the harness implementation itself:
  `evals/src/{main,base,case,run,grade,judge,diagnose,report,style}.rs` in
  the remote repo — not reproduced here in full since it's a large,
  self-contained program; read those files directly from
  <https://github.com/hodstack/hodstack/tree/main/evals/src> if
  implementing this, starting with `case.rs` (grader definitions) and
  `run.rs` (orchestration) as the two files that define the case format
  this doc describes.
- Before investing in this: confirm this repo actually has (or plans to
  gain) multiple skills with prose-sensitive behavior worth regression
  testing — `skills/deps-upgrade` already exists locally, so at minimum
  one real skill is available to write a first case against.
