# Gap docs: local `hod` vs `hodstack/hodstack` on GitHub

This directory documents features and infrastructure that exist in the
upstream repo (<https://github.com/hodstack/hodstack>) but not in this local
copy. It was produced by diffing this working tree against a fresh clone of
the remote `main` branch on 2026-09-01.

The local repo has no git history and no relationship to the GitHub repo —
it's an earlier, standalone scaffold (`hodstack` crate, flat layout: `src/`,
`templates/`, `skills/`) that predates the upstream project being renamed to
`hod`, split into a Cargo workspace (`cli/`, `evals/`), and given real
installers, CI, and a multi-agent CLI.

Each file below covers one feature/improvement: what it does, why it exists,
and enough code (lifted from the remote repo at the paths cited) for an agent
to implement it here without re-cloning the remote. Where a remote path is
given as `cli/src/...`, the equivalent local path — assuming this repo stays
a flat single crate rather than adopting the `cli/` workspace split — is
`src/...`.

## Reading order

Roughly most-foundational first; later docs build on earlier ones.

1. [`01-clap-cli-parsing.md`](01-clap-cli-parsing.md) — replace hand-rolled
   `env::args()` parsing with `clap`. Everything else below assumes this is
   done first.
2. [`02-agent-detection.md`](02-agent-detection.md) — detect and launch
   whichever coding agent (`claude`, `codex`, `cursor-agent`, `opencode`,
   `gemini`) is on `PATH`, instead of a single fixed invocation.
3. [`03-list-command.md`](03-list-command.md) — `hod list`.
4. [`04-update-command.md`](04-update-command.md) — `hod update`: self-update
   the binary and re-sync project files, with `--check`/`--project`/`--force`.
5. [`05-init-flow.md`](05-init-flow.md) — `hod init` refuses to clobber an
   existing `AGENTS.md`/`CLAUDE.md` and hands off to the detected agent.
6. [`06-completions-command.md`](06-completions-command.md) — `hod
   completions <shell>`.
7. [`07-update-notice.md`](07-update-notice.md) — background "a newer hod is
   available" notice after commands.
8. [`08-installers-packaging.md`](08-installers-packaging.md) —
   `install.sh` / `install.ps1` / npm package for one-line installs.
9. [`09-build-quality-lints.md`](09-build-quality-lints.md) — clippy/rustfmt
   config, release profile tuning, `cargo-make` task graph.
10. [`10-testing-infrastructure.md`](10-testing-infrastructure.md) —
    snapshot tests (`snapbox`) and supply-chain checks (`cargo-deny`,
    `cargo-vet`, `cargo-audit`, `cargo-machete`).
11. [`11-evals-harness.md`](11-evals-harness.md) — an eval framework that
    runs a real coding agent against seeded projects and grades the result,
    to regression-test skill text.
12. [`12-ci-cd-and-repo-docs.md`](12-ci-cd-and-repo-docs.md) — GitHub Actions
    CI/release workflows, Dependabot, `SECURITY.md`, `CONTEXT.md`,
    `AGENTS.md`-writing conventions, and branding assets.

## Ground rule for implementing any of these

The remote repo's own `AGENTS.md` (quoted in doc 12) states its house style
directly: no comments in code, no defensive error handling for cases that
can't happen, split long functions instead of commenting them, and write
messages to the user in short imperative sentences. The code excerpts in
these docs already follow that style — keep matching it.
