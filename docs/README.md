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

1. ✅ [`01-clap-cli-parsing.md`](01-clap-cli-parsing.md) — **Done.** Replaced
   hand-rolled `env::args()` parsing with `clap`. Everything else below
   assumes this is done first.
2. ✅ [`02-agent-detection.md`](02-agent-detection.md) — **Done.** Detect and
   launch whichever coding agent (`claude`, `codex`, `cursor-agent`,
   `opencode`, `gemini`) is on `PATH`, instead of a single fixed invocation.
   See `src/agent.rs`.
3. ✅ [`03-list-command.md`](03-list-command.md) — **Done.** `hod list`. See
   `src/list.rs`.
4. ✅ [`04-update-command.md`](04-update-command.md) — **Done.** `hod
   update`: self-update the binary and re-sync project files, with
   `--check`/`--project`/`--force`. See `src/update.rs` and `Mode` in
   `src/sync.rs`. The `HOD_COMMIT` compile-time piece this depends on for
   an accurate "am I current?" check is tracked separately — see doc 13.
5. ✅ [`05-init-flow.md`](05-init-flow.md) — **Done.** `hod init` refuses to
   clobber an existing `AGENTS.md`/`CLAUDE.md` and hands off to the detected
   agent via a new shipped `init` skill. See `init()` in `src/lib.rs` and
   `skills/init/SKILL.md`.
6. ✅ [`06-completions-command.md`](06-completions-command.md) — **Done.**
   `hod completions <shell>` (real `clap_complete::generate` body, not a
   stub) plus the styled `--help` template. See `src/help.rs` and
   `Command::Completions` in `src/cli.rs`.
7. ✅ [`07-update-notice.md`](07-update-notice.md) — **Done.** Background "a
   newer hod is available" notice after commands, skipped for `update`/
   `completions`. See `update::notice` in `src/update.rs`.
8. ✅ [`08-installers-packaging.md`](08-installers-packaging.md) — **Done.**
   `install.sh` / `install.ps1` / `npm/` package, mirroring the release
   slug already baked into `src/update.rs`. Inert until a real release
   pipeline (doc 12/13) actually publishes `hod-<target>.tar.gz` +
   `checksums.txt` + `version.txt`.
9. ✅ [`09-build-quality-lints.md`](09-build-quality-lints.md) — **Done.**
   `[lints]` in `Cargo.toml` (workspace split not adopted — single crate),
   `rustfmt.toml`, `rust-toolchain.toml`, `clippy.toml`, `Makefile.toml`.
   Clean under `cargo clippy --all-targets --all-features -- -D warnings`
   and `cargo fmt --all -- --check`.
10. ✅ [`10-testing-infrastructure.md`](10-testing-infrastructure.md) —
    **Done.** `tests/cli.rs` (snapshot + subprocess integration tests via
    `snapbox`), `deny.toml`, and a `cargo vet init`-seeded `supply-chain/`.
    Verified locally: `cargo deny check`, `cargo vet check`, `cargo audit`,
    `cargo machete`, and `typos` all pass clean.
11. ✅ [`11-evals-harness.md`](11-evals-harness.md) — **Done, scoped down.**
    A working harness under `evals/` (its own Cargo project, per the doc's
    isolation rule) with the case format and mechanical graders, but no LLM
    judge and no real coding-agent driver — see `evals/README.md` for
    exactly what's simplified vs. upstream and why.
12. ✅ [`12-ci-cd-and-repo-docs.md`](12-ci-cd-and-repo-docs.md) — **Done,
    scoped down.** `.github/workflows/ci.yml` (adapted to this repo's
    actual `cargo-make` tasks in `Makefile.toml`, not upstream's `cli:`
    prefix), `.github/dependabot.yml`, `SECURITY.md`, `CONTEXT.md`,
    `AGENTS.md`. Branding assets (`art/`) skipped — no logic to port and
    this repo has no visual identity yet.
13. ✅ [`13-hod-commit-and-ci.md`](13-hod-commit-and-ci.md) — **Done.**
    `.github/workflows/release.yml` sets `HOD_COMMIT` on the `cargo build`
    step. `src/lib.rs`'s `version()` already carried the `OnceLock`/
    `option_env!("HOD_COMMIT")` form from doc 04's port, so no code change
    was needed there — this doc's remaining piece was purely the workflow.

## Ground rule for implementing any of these

The remote repo's own `AGENTS.md` (quoted in doc 12) states its house style
directly: no comments in code, no defensive error handling for cases that
can't happen, split long functions instead of commenting them, and write
messages to the user in short imperative sentences. The code excerpts in
these docs already follow that style — keep matching it.
