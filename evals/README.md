# evals

A scoped-down implementation of `docs/11-evals-harness.md`'s eval harness —
see that doc for the design this is based on, and `AGENTS.md` for the case
format this crate actually implements.

## What's simplified vs. upstream

The upstream harness (`hodstack/hodstack`'s `evals/`) is a much larger
program. This version keeps its case format and grader set but cuts:

- **No judge model.** Upstream grades a case two ways: mechanical graders
  (TOML front matter) and a separate judge model that reads the agent's own
  report against the Markdown expectation. This crate only runs the
  mechanical graders. The expectation text is carried through `test.md` and
  printed on a failing case, but nothing grades it.
- **No real coding-agent driver.** Upstream actually drives `claude`/`codex`
  etc. against a case and records their real tool-call transcript. This
  crate has no such integration — it shells out to whatever command
  `HOD_EVAL_AGENT_CMD` names, and that command is responsible for both doing
  the task and (if any `tool_used`/`tool_order` grader is present) writing
  its own trace file in the small JSONL contract documented in `AGENTS.md`
  section 3. Wiring `HOD_EVAL_AGENT_CMD` to a real agent that emits that
  trace format is future work.
- **No copy-on-write base cloning.** A base is seeded once per invocation
  into `target/eval-cache/<name>`, then plain-copied (not COW-cloned) into
  each case's temporary working directory. Slower than upstream at scale,
  fine for the handful of cases here.
- **No diagnosis file / styled report.** A failing run prints which graders
  failed, their detail, and the case's expectation text to stdout — no
  `.runs/diagnosis/` file written for handing to a fixer agent.
- **One synthetic base, one case.** `bases/fake-php/` is a two-file
  synthetic project (no real Composer/Laravel), and
  `deps-upgrade/raises-the-minor/` is the only case, proving the harness
  end-to-end rather than covering `deps-upgrade` thoroughly. `tests/
  harness.rs` runs it against two fixture shell scripts (one that does the
  upgrade and writes a trace line, one that does nothing) standing in for a
  real agent, confirming the harness passes the first and fails the second.

## Running it

```sh
cargo run -- --list
cargo run -- deps-upgrade/raises-the-minor
HOD_EVAL_AGENT_CMD=/path/to/some/agent/wrapper.sh cargo run -- deps-upgrade
```

Without `HOD_EVAL_AGENT_CMD` set, every case fails immediately with `no
agent command configured; set HOD_EVAL_AGENT_CMD` — there is nothing this
harness can run in its place.

## Testing the harness itself

```sh
cargo test
```

Unit tests cover the TOML case parser and each grader in isolation.
`tests/harness.rs` runs the compiled binary against the one real case using
two fixture agent scripts (Unix only), to prove a passing and a failing run
both work end-to-end.
