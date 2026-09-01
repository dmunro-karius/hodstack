# CI/CD, Dependabot, and repo-level docs

## What's missing

No `.github/` at all (no CI, no release automation, no Dependabot), and none
of `SECURITY.md`, `CONTEXT.md`, or an `AGENTS.md` describing how this repo
itself should be written and maintained. Also no branding assets (`art/`).

## What the remote repo does

### CI (`.github/workflows/ci.yml`)

Runs on every push to `0.x` and every PR. Jobs (each independent, each its
own `runs-on`):

- **`rust`** — `make cli:lint` (fmt + clippy, see `09-build-quality-lints.md`),
  `make cli:docs` (rustdoc with `-D warnings`), `make cli:unit` (tests),
  `make cli:build`.
- **`msrv`** — pins toolchain to the exact MSRV (`1.85.0`) and runs `make
  cli:msrv` (`cargo +1.85.0 check`) to catch code that accidentally needs a
  newer compiler than promised in `Cargo.toml`'s `rust-version`.
- **`skills`** — `make skills:lint`, i.e. `npx markdownlint-cli2` against
  every `skills/**/*.md` using `.markdownlint.json`.
- **`typos`** — `make typos` via the `typos` crate/CLI, checked across the
  whole repo (`typos.toml` configures ignored words/paths).
- **`audit`**, **`deny`**, **`vet`**, **`machete`** — the four supply-chain
  jobs from `09-build-quality-lints.md` / `10-testing-infrastructure.md`,
  each its own job so a failure is attributable at a glance in the PR
  checks list.
- **`coverage`** — `cargo-llvm-cov`.

All jobs pin third-party actions to a full commit SHA with a version
comment (e.g. `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 #
v7.0.1`) rather than a floating tag — standard supply-chain hardening for
GitHub Actions.

### Release (`.github/workflows/release.yml`)

Triggered by `workflow_run` after CI succeeds on `0.x`, or manually
(`workflow_dispatch`). Three sequential jobs:

1. **`build`** — a matrix over 5 targets (`aarch64`/`x86_64` ×
   apple-darwin, `x86_64`/`aarch64` × unknown-linux-musl,
   `x86_64-pc-windows-msvc`), each producing `hod-<target>.tar.gz`
   containing the binary + `LICENSE.md`. `HOD_COMMIT` env var is set to the
   triggering commit SHA (this is what `04-update-command.md`'s
   `version()`/`Build` code embeds and compares against).
2. **`publish`**: downloads all 5 archives, computes `checksums.txt`
   (`sha256sum hod-*.tar.gz`), writes `version.txt` (`"<crate-version>
   <sha>\n"`), and creates a GitHub Release tagged `edge-<UTC
   timestamp>`, marked `--latest`, attaching the archives plus
   `install.sh`/`install.ps1` (see `08-installers-packaging.md` — these
   scripts are published *as release assets*, which is what the
   `.../releases/latest/download/install.sh` URL resolves to). It then
   deletes every `edge-*` release past the 10 most recent, to avoid
   unbounded release-list growth.
3. **`npm`**: only runs if an `NPM_TOKEN` secret is set (skips cleanly
   otherwise — note the `if: env.NPM_TOKEN != ''` guard on every step, not
   just the job, since GitHub Actions can't skip a whole job on a secret
   check alone). Publishes the npm package (see `08-installers-packaging.md`)
   under three package names (`hodstack`, `@hodstack/cli`, `@hodstack/hod`)
   with an `edge` dist-tag, versioned as `<crate-version>-edge.<timestamp>`.

Key policy embedded here: **every push to `0.x` cuts a new release** (an
"edge" channel model, not semver-tagged releases gating manual review) —
the crate version in `Cargo.toml` doesn't need to bump for this to work,
since the release tag/npm dist-tag carry the timestamp instead.

### Dependabot (`.github/dependabot.yml`)

```yaml
version: 2
updates:
  - package-ecosystem: cargo
    directory: "/cli"
    target-branch: "0.x"
    schedule:
      interval: weekly
    cooldown:
      default-days: 5
    open-pull-requests-limit: 10
    groups:
      patch-and-minor:
        update-types: [patch, minor]

  - package-ecosystem: github-actions
    directory: "/"
    target-branch: "0.x"
    schedule:
      interval: weekly
    cooldown:
      default-days: 5
    groups:
      github-actions:
        patterns: ["*"]
```

Two ecosystems watched (`cargo`, `github-actions`), each grouped into one
PR per week rather than one PR per dependency, and each with a 5-day
`cooldown` (waits 5 days after a new version is published before proposing
it — avoids being first to pull a just-yanked release).

### `SECURITY.md`

```markdown
# Security Policy

**PLEASE DON'T DISCLOSE SECURITY-RELATED ISSUES PUBLICLY, [SEE BELOW](#reporting-a-vulnerability).**

## Reporting a Vulnerability

If you discover a security vulnerability, please report it privately using one of the following channels:

1. **GitHub Private Vulnerability Reporting** (preferred) — go to the repository's **Security** tab and click **"Report a vulnerability"**. This creates a private advisory visible only to maintainers and provides a structured workflow for triage, fix coordination, and CVE assignment.

2. **Email** — send the details to [Nuno Maduro] at **enunomaduro@gmail.com**.

All security vulnerabilities will be promptly addressed.
```

Adapt the name/email to this project's actual maintainer before reusing.

### `CONTEXT.md` — a repo-wide glossary

A long, deliberately exhaustive terminology document defining every
domain noun used across the codebase (`coding agent`, `model`, `client`,
`skill directory`, `skill`, `user skill`, `model skill`, `shipped skill`,
`project skill`, `installed skill`, `vendored skill`, `skills tree`,
`project`, `project file`, `intention`, `rule`, `rule file`, `lock`,
`ownership`, `sync`, `build`, `release`, `opening prompt`), each with an
`_Avoid_:` line listing near-synonym terms *not* to use, plus a
"Relationships" section and a "Flagged ambiguities" section documenting
*why* certain words were disambiguated (e.g. "agent" used to mean four
different things across the codebase; resolved into **coding agent**,
**model**, `AGENTS.md`, and **skill directory** respectively).

This is worth adopting in spirit even if the exact terms differ: as a
project's domain vocabulary grows, a single glossary file that pins one
word to one meaning (and explicitly rules out the tempting synonyms)
prevents the same concept from accreting three different names across
`README.md`, code comments, and `AGENTS.md` over time.

### `AGENTS.md` — how *this repository itself* should be written

Not a user-facing doc — it's instructions for a coding agent working on
the `hodstack` repo itself, and it's unusually opinionated. Highlights:

- **`AGENTS.md` files are read by agents, never by users.** Public text
  (`README.md`, website, release notes, manifest `description` fields) is
  governed by a separate section (§3) with a different voice: calm,
  concrete, no superlatives, no unverified claims, "one dry joke per page
  at most, do not explain it."
- **Style for `AGENTS.md`/rule prose**: [ASD-STE100 Simplified Technical
  English](https://www.asd-ste100.org) — one instruction per sentence,
  imperative mood, active voice, one meaning per word, no contractions, no
  synonym variation. GitHub-flavored Markdown, one `#` heading per file,
  one paragraph per line (keeps diffs small), numbered sections so other
  files can cross-reference ("see section 3").
- **A rule only exists to record a fault that already happened** — "name
  the decision that you made wrong, or name the decision that these files
  refused to give you. A wrong change that you imagine is not a fault."
  Corrections from a user *are* rules and need no further justification;
  everything else needs a real incident behind it.
- **Never restate what the code already shows.** "Write no sentence for a
  fact that an agent finds when it reads the code... Put the fact in the
  code first, because a test stops the agent that breaks it and a sentence
  here does not."
- **Code style**: no comments at all — "Delete each comment that you find
  in a file that you change" — express intent through naming and types
  instead ("Give each value a type that makes a wrong value impossible").
  This matches the code excerpted throughout this doc set, which has zero
  inline comments.
- **Maintenance discipline**: a rule that becomes obsolete is replaced in
  place, not appended alongside; git history is the changelog, not a
  "removed: ..." comment left in the file.

### Branding (`art/`)

SVG + PNG assets for app icon, favicon, wordmark (light/dark/inverse
variants), lockup (horizontal/stacked), badge, and a social banner — used
by the npm package listing, GitHub repo social preview, and any future
website. Not code; copy the directory structure
(`art/{app-icon,badge,favicon,icon,lockup,png,social,wordmark}/`) as a
placeholder if/when this project wants its own visual identity, but
there's no logic to port.

## Implementation notes for this repo

- CI/release automation (`.github/workflows/`) is infrastructure, not
  code — it can be added at any point once there's a `main`/release branch
  strategy decided, but the release workflow specifically has a hard
  dependency on `08-installers-packaging.md`'s expected artifact layout
  (`hod-<target>.tar.gz` + `checksums.txt` + `version.txt`) and
  `04-update-command.md`'s `HOD_COMMIT` embedding — implement those first,
  or at least design them together.
- `SECURITY.md` and `CONTEXT.md` are pure documentation — cheapest items
  in this entire doc set to add, no code dependency at all. `CONTEXT.md`
  in particular is worth writing early, before this repo's own vocabulary
  (skill, project, sync, lock, etc. — this repo already uses several of
  these words) drifts the way the remote's own `CONTEXT.md` documents it
  once did.
- Adopting the `AGENTS.md` conventions (no comments, ASD-STE100 for
  agent-facing docs, "only write a rule after a real fault") is a
  standards decision for this repo's maintainers, not something to import
  wholesale without agreement — flag it for discussion rather than
  silently changing this repo's existing conventions.
