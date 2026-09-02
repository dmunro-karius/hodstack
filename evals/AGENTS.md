# evals

Regression-tests skill *text* — the prose in `skills/<name>/SKILL.md` — by
running an agent against a seeded project and checking what it did and what
it left behind. Its own `[workspace]`/`Cargo.toml`: never a member of the
root crate's build, so its dependencies never reach `cargo vet`/`cargo
deny`/`cargo machete`/the MSRV check of `hod`, and never ship in the crate a
user installs.

## 1. Directory shape

```
<skill>/<case>/
├── setup.sh      plant the precondition of this case
├── test.md       the graders in the front matter, the expectation in the body
└── fixtures/     files of this case, written over the base (optional)
```

A skill is any top-level directory here (other than `src`, `bases`,
`target`) holding at least one case directory. A case directory is any
directory holding a `test.md`.

`bases/<name>/seed.sh` plants one reusable starting project. It's run once
per `evals` invocation, cwd already set to an empty cache directory
(`target/eval-cache/<name>`); it writes files directly into `.` and writes
no `.git` — the harness does `git init`, commits the base, then runs
`setup.sh`. A `setup.sh` that wants a real diff in the working tree commits
its own precondition change explicitly, rather than the case starting
already-dirty by accident.

## 2. `test.md`

TOML front matter between `+++` lines, then a Markdown body:

```
+++
base = "fake-php"
intent = "Upgrade the dependencies of this project to newer versions."
allowed_tools = ["Bash", "Read", "Write", "Edit", "Grep"]

[[graders]]
type = "file_content"
path = "composer.json"
pattern = "1\\.0\\.0"
match = "not_contains"

[[graders]]
type = "tool_used"
tool = "Edit"
+++

The agent reports that it raised `acme/example` to a newer minor version.
```

`base` names a directory under `bases/`. `threshold` (default `1.0`) is the
pass rate a case needs across `--runs` repetitions.

### Graders

Mechanical, no model involved — a case passes only if every grader passes:

- `tool_used` — `tool`, optional `input_match` (regex against the recorded
  input of that tool call).
- `tool_order` — `tools`, a list; passes if the trace reaches every name in
  that order (gaps allowed, reordering not).
- `file_content` / `regex` — `path`, `pattern` (regex), optional `match`
  (`contains`, the default, or `not_contains`). Both read the same file;
  `regex` is kept as a separate grader name for parity with the upstream
  spec, even though this harness gives it no different target.
- `file_exists` — `path`.
- `git_clean` / `git_dirty` — the working tree's `git status --porcelain`.
- `head_unmoved` — `git rev-parse HEAD` is unchanged from right before the
  agent ran (setup.sh's own commits don't count against this).

### Expectation

The Markdown body is not graded by this harness (see `README.md` — no judge
model is wired up here). It's echoed in a failing case's report so a human
reading the failure still sees what the case was actually trying to prove.

## 3. The agent contract

This harness has no real coding-agent integration. It runs whatever shell
command is named by `HOD_EVAL_AGENT_CMD` inside the case's working copy, and
gives it three environment variables:

- `HOD_EVAL_INTENT` — the case's `intent` string.
- `HOD_EVAL_ALLOWED_TOOLS` — the case's `allowed_tools`, comma-joined.
- `HOD_EVAL_TRACE_FILE` — a path. If the command wants `tool_used` /
  `tool_order` graders to see anything, it appends one JSON object per line
  there: `{"tool": "Edit", "input": "composer.json"}`.

A case with no `HOD_EVAL_AGENT_CMD` set fails immediately with that message
— see `README.md` for why this repo can't supply a real one out of the box.

## 4. Running it

```sh
cargo run -- --list                 # name every case, run nothing
cargo run --                        # run every case of every skill
cargo run -- deps-upgrade           # run one skill
cargo run -- deps-upgrade/raises-the-minor   # run one case
cargo run -- deps-upgrade --runs 3  # repeat each matched case 3 times
```

Exits non-zero if any case fails. A failing case prints which graders
failed and why, plus the expectation text.
