# `HOD_COMMIT`: making self-update's version check actually work

## What's missing

`04-update-command.md` ported `update.rs` with this function:

```rust
fn running() -> Option<&'static str> {
    option_env!("HOD_COMMIT")
}
```

`option_env!` is a compile-time macro — it reads an environment variable
that was set in the process that ran `rustc`/`cargo build`, not one read at
runtime. Nothing in this repo ever sets `HOD_COMMIT` when building, so
`running()` always returns `None`. `update::update()` compares it against
the newest release's commit (`running() == Some(newest.commit.as_str())`);
since `None` never equals `Some(_)`, **every locally built `hod` binary
believes it is out of date**, forever, even the instant after it installed
the newest build. `hod update --check` will never print `Current`, and a
plain `hod update` will re-download and re-swap the binary every single
time it runs.

There is also no `.github/workflows/` directory in this repo at all yet, so
there is no release pipeline to set `HOD_COMMIT` in the first place, and no
release for `hod update`'s default `HOD_RELEASE_URL` to actually fetch from
(you've been testing it locally against `HOD_RELEASE_URL` file fixtures —
see the manual verification done for `04-update-command.md`).

This doc covers **only** wiring `HOD_COMMIT` through a release CI pipeline.
It assumes `01`–`04` are already implemented (`src/update.rs`, `src/sync.rs`
`Mode`, `src/cli.rs`'s `Update` variant, `agent::find`/`start`, etc. already
exist). Nothing in `src/update.rs` needs to change — `running()` is already
correct code, it just never receives a value. `08-installers-packaging.md`
and `12-ci-cd-and-repo-docs.md` cover related ground (release *artifact
layout*, and CI *jobs* more broadly) but neither one, on its own, hands you
a working `HOD_COMMIT`-setting workflow — this doc does.

## The mechanism, confirmed against the live remote repo

`hodstack/hodstack` on GitHub is a real, continuously-released project (edge
builds roughly every push, e.g. `edge-20260824-093515`), so this isn't
speculative — it's exactly what's running today. There is **no `build.rs`
step** that computes or embeds the commit. It is nothing more than a plain
environment variable set on the CI step that invokes `cargo build`:

`.github/workflows/release.yml` (build job, relevant step):

```yaml
      - name: Build
        run: make cli:build TARGET=${{ matrix.target }}
        env:
          HOD_COMMIT: ${{ github.event_name == 'workflow_dispatch' && github.sha || github.event.workflow_run.head_sha }}
```

`make cli:build` in turn is just:

```makefile
cli\:build:
	cd cli && cargo build --locked --release $(if $(TARGET),--target $(TARGET))
```

That's it — `cargo build` inherits `HOD_COMMIT` from the shell environment
the CI step set, and `option_env!("HOD_COMMIT")` picks it up at compile
time. **This repo has no `cli/` subdirectory and no `make`/`cargo-make`
wiring yet** (`09-build-quality-lints.md`/`10-testing-infrastructure.md`
aren't implemented), so port this as a direct `cargo build` invocation from
the repo root, not through `make`.

Confirmed also from the live repo: local/dev builds upstream **never** set
`HOD_COMMIT` either — their own `cli:run` task (`cd cli && cargo run -q --
...`) has no `HOD_COMMIT` in its `env`. Upstream's own contributors get the
same "always considers itself outdated" behavior on `cargo run`/`cargo
build` that you saw locally. `HOD_COMMIT` is a **release-CI-only** concern.
Don't try to make plain local `cargo build` embed it — that's not how
upstream does it, and it's not what this doc is asking you to add.

Upstream also uses the commit to enrich `hod --version`, via a small
`OnceLock` in `cli/src/lib.rs` (this repo's `src/lib.rs` currently just has
a bare `pub fn version() -> &'static str { env!("CARGO_PKG_VERSION") }` from
`04-update-command.md` — that's fine for `update.rs`'s internal use, but
doesn't show the commit to the user anywhere). Upstream's version:

```rust
fn version() -> &'static str {
    static VERSION: OnceLock<String> = OnceLock::new();

    VERSION
        .get_or_init(|| {
            let version = env!("CARGO_PKG_VERSION");

            match option_env!("HOD_COMMIT") {
                Some(commit) => format!("{version} ({})", commit.get(..7).unwrap_or(commit)),
                None => version.to_owned(),
            }
        })
        .as_str()
}
```

Port this too — it's the same `option_env!("HOD_COMMIT")` mechanism this
doc is wiring up, and it's what makes `hod --version` show `0.1.0
(3f2b1c9)` on a release build instead of a bare `0.1.0`.

## The release workflow, in full (adapted for this repo's layout)

Fetched directly from the live `hodstack/hodstack` repo
(`.github/workflows/release.yml`), then adapted below for this repo:
single flat crate at the repo root (no `cli/` subdir), branch `main` (not
upstream's `0.x`), no `make`/`cargo-make` (invoke `cargo` directly), no
`ci.yml` to chain off of yet (trigger on push to `main` and
`workflow_dispatch` directly), no npm package (skip that job —
`08-installers-packaging.md` territory), license file named `LICENSE` not
`LICENSE.md`.

`.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    branches: [main]
  workflow_dispatch:

permissions:
  contents: write

concurrency:
  group: release
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.runner }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: aarch64-apple-darwin
            runner: macos-latest
          - target: x86_64-apple-darwin
            runner: macos-latest
          - target: x86_64-unknown-linux-musl
            runner: ubuntu-latest
          - target: aarch64-unknown-linux-musl
            runner: ubuntu-24.04-arm
          - target: x86_64-pc-windows-msvc
            runner: windows-latest
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1

      - uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
        with:
          toolchain: stable
          targets: ${{ matrix.target }}

      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2
        with:
          key: ${{ matrix.target }}

      - name: Install the musl toolchain
        if: runner.os == 'Linux'
        run: sudo apt-get update && sudo apt-get install --yes musl-tools

      - name: Build
        run: cargo build --locked --release --target ${{ matrix.target }}
        env:
          HOD_COMMIT: ${{ github.sha }}

      - name: Pack
        shell: bash
        run: |
          set -eu
          name=hod
          if [ "${{ runner.os }}" = "Windows" ]; then name=hod.exe; fi
          staging="$(mktemp -d)"
          cp "target/${{ matrix.target }}/release/$name" "$staging/$name"
          cp LICENSE "$staging/LICENSE"
          mkdir -p dist
          tar -czf "dist/hod-${{ matrix.target }}.tar.gz" -C "$staging" "$name" LICENSE

      - uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7.0.1
        with:
          name: hod-${{ matrix.target }}
          path: dist/hod-${{ matrix.target }}.tar.gz
          retention-days: 1

  publish:
    name: Publish release
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1

      - uses: actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c # v8.0.1
        with:
          path: dist
          merge-multiple: true

      - name: Write version.txt
        run: |
          set -eu
          crate="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)"
          stamp="$(date -u +%Y%m%d-%H%M%S)"
          echo "tag=edge-$stamp" >> "$GITHUB_ENV"
          printf '%s %s\n' "$crate" "$GITHUB_SHA" > dist/version.txt

      - name: Write the checksums
        working-directory: dist
        run: sha256sum hod-*.tar.gz > checksums.txt

      - name: Write the release
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          set -eu
          notes="$(printf '%s\n' \
            "The build of \`main\`, at commit \`${GITHUB_SHA}\`." \
            "" \
            "That address gives the newest build. Every push to \`main\` writes a release and moves it.")"
          gh release create "$tag" --title "$tag" --notes "$notes" --latest --target "$GITHUB_SHA" \
            dist/*

      - name: Delete each release after the tenth
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          set -eu
          gh release list --limit 100 --json tagName,createdAt \
            | jq -r 'sort_by(.createdAt) | reverse | .[].tagName | select(startswith("edge-"))' \
            | tail -n +11 \
            | while read -r old; do gh release delete "$old" --yes --cleanup-tag; done
```

## Design points worth keeping

- **No `build.rs`, no code change in `update.rs`.** `HOD_COMMIT` is purely
  an environment variable on the CI step that runs `cargo build`. Resist
  the urge to compute it in Rust (e.g. shelling out to `git` from a build
  script) — that would bake the *builder's* working-tree state into the
  binary in a way that's harder to audit than a one-line CI `env:`, and it's
  not what upstream does.
- **`version.txt`'s format is load-bearing**: `"<crate-version> <full 40-char
  sha>\n"`, exactly what `update.rs`'s `Build::read()` (already ported)
  expects — `version.split_whitespace()`, first field version, second field
  commit. Use the **full** SHA in `version.txt`/the release, not a
  shortened one — `update.rs`'s `short()` truncates it for display only.
- **`checksums.txt` must be computed after all 5 artifacts are downloaded**,
  in the `publish` job, not per-target in `build` — `download.rs`'s
  `wanted_sum()` (already ported) expects one `checksums.txt` covering every
  target's `.tar.gz`.
- **Pin third-party Actions to a full commit SHA** with a version comment
  (`actions/checkout@3d3c42e5... # v7.0.1`), not a floating tag — standard
  supply-chain hardening, and matches what `12-ci-cd-and-repo-docs.md`
  already flags as a convention worth adopting.
- **Prune old releases** (keep the 10 most recent `edge-*`) so the release
  list doesn't grow unbounded on every push.
- **`permissions: contents: write`** at the workflow level — needed for
  `gh release create`/`gh release delete`.
- Upstream triggers release via `workflow_run` chained off a separate
  `ci.yml` job succeeding first. This repo doesn't have `ci.yml` yet
  (`12-ci-cd-and-repo-docs.md`), so this doc's version triggers directly on
  `push: branches: [main]` instead. **Revisit this** once `ci.yml` exists —
  chaining release off CI success (rather than off every push) avoids
  publishing a release built from code that fails its own tests.

## Implementation notes for this repo

- Add `.github/workflows/release.yml` with the content above.
- Port `version()`'s `OnceLock` form (shown above) into `src/lib.rs`,
  replacing the current bare `env!("CARGO_PKG_VERSION")` version. Wire it
  into the `clap` `Cli` the way `01-clap-cli-parsing.md`/upstream does —
  `Cli::command().version(version())` — so `hod --version` shows the commit
  too, not just the internal `update.rs` check.
- No changes needed to `src/update.rs` — `running()` is already correct;
  it just needs `HOD_COMMIT` to actually be set by whatever built the
  binary, which is what the workflow above does.
- **This only affects binaries built by this new release workflow.** A
  developer's own `cargo build`/`cargo run` will still report itself as
  outdated forever, same as upstream's own contributors experience — that's
  expected and not a bug to fix here.
- To verify the wiring end-to-end without cutting a real GitHub release:
  build locally with `HOD_COMMIT` set by hand
  (`HOD_COMMIT=$(git rev-parse HEAD) cargo build --release`), then point
  `HOD_RELEASE_URL` at a local fixture directory whose `version.txt`
  contains that same SHA (`printf '%s %s\n' "$(cargo metadata --no-deps --format-version=1 | jq -r '.packages[0].version')" "$(git rev-parse HEAD)" > version.txt`)
  and confirm `./target/release/hod update --check` prints `Current`, not
  `Newest`. This is the same manual-fixture technique used to verify
  `04-update-command.md`'s download/checksum/swap path.
- `LICENSE` in this repo has no `.md` extension (unlike upstream's
  `LICENSE.md`) — the `Pack` step above already accounts for this; don't
  copy upstream's `cp ../LICENSE.md` line verbatim.
- Skip the `npm` publish job entirely for now — this repo has no `npm/`
  package (`08-installers-packaging.md`), so there's nothing for it to
  publish.
