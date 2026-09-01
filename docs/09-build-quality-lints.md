# Build quality: lints, release profile, `cargo-make` tasks

## What's missing

Local `Cargo.toml` has no `[lints]` section at all and no tuned
`[profile.release]` — any `unwrap()`, `panic!()`, or stray `println!`
compiles clean. There's also no task runner (`cargo-make`) codifying how to
lint/test/build consistently.

## What the remote repo does

### `Cargo.toml` lint gate (workspace-level, `cli/Cargo.toml`)

```toml
[workspace]
resolver = "3"
exclude = [".agents", ".claude"]

[package]
name = "hod"
version = "0.0.1"
edition = "2024"
rust-version = "1.85"
...

[lints]
workspace = true

[workspace.lints.rust]
unsafe_code = "forbid"
missing_debug_implementations = "warn"
unused_qualifications = "warn"
unexpected_cfgs = { level = "deny", check-cfg = [] }

[workspace.lints.rustdoc]
broken_intra_doc_links = "deny"
private_intra_doc_links = "warn"

[workspace.lints.clippy]
correctness = { level = "deny", priority = -1 }
suspicious = { level = "deny", priority = -1 }
style = { level = "warn", priority = -1 }
complexity = { level = "warn", priority = -1 }
perf = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
cargo = { level = "warn", priority = -1 }

missing_errors_doc = "allow"
missing_panics_doc = "allow"
module_name_repetitions = "allow"
must_use_candidate = "allow"
multiple_crate_versions = "allow"

unwrap_used = "deny"
expect_used = "warn"
panic = "deny"
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
print_stdout = "deny"
print_stderr = "deny"
exit = "deny"

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
```

Notable individual denies worth calling out:

- `unwrap_used = "deny"` and `panic = "deny"` — force real error handling
  (`anyhow::Result` + `?`/`bail!`/`context()`) instead of crashing. Tests
  are exempted via `cli/clippy.toml`:

  ```toml
  allow-unwrap-in-tests = true
  allow-expect-in-tests = true
  allow-panic-in-tests = true
  allow-print-in-tests = true

  doc-valid-idents = ["..", "Hodstack", "GitHub"]
  ```

- `print_stdout = "deny"` / `print_stderr = "deny"` — forces all output
  through the `out: &mut impl Write` / `err: &mut impl Write` parameter
  pattern seen throughout every module excerpted in these docs (`list.rs`,
  `init.rs`, `update.rs`, ...), rather than ad hoc `println!`. This is
  *why* every command function in the remote takes an `out: &mut impl
  Write` argument — it's not incidental style, it's load-bearing for
  testability (tests capture output into a `Vec<u8>`, see
  `10-testing-infrastructure.md`) and for this lint.
- `unsafe_code = "forbid"` — hard `forbid`, not `deny`, so it can't be
  locally overridden with `#[allow(...)]`.

### `cli/rustfmt.toml`

```toml
edition = "2024"
style_edition = "2024"
newline_style = "Unix"
```

### `cli/rust-toolchain.toml`

```toml
[toolchain]
channel = "stable"
profile = "minimal"
components = ["clippy", "rustfmt"]
```

### `cli/Makefile.toml` (cargo-make task graph)

```toml
[config]
default_to_workspace = false
skip_core_tasks = true

[env]
CARGO_TERM_COLOR = "always"

[tasks.lint]
description = "Auto-fix formatting and clippy issues"
dependencies = ["lint-fmt", "lint-clippy"]

[tasks.lint-fmt]
command = "cargo"
args = ["fmt", "--all"]

[tasks.lint-clippy]
command = "cargo"
args = ["clippy", "--all-targets", "--all-features", "--fix", "--allow-dirty", "--allow-staged", "--", "-D", "warnings"]

[tasks."test:lint"]
description = "Verify formatting and clippy without modifying files"
dependencies = ["test-fmt", "test-clippy"]

[tasks.test-fmt]
command = "cargo"
args = ["fmt", "--all", "--", "--check"]

[tasks.test-clippy]
command = "cargo"
args = ["clippy", "--all-targets", "--all-features", "--locked", "--", "-D", "warnings"]

[tasks."test:unit"]
command = "cargo"
args = ["test", "--all-features", "--locked"]

[tasks."test:docs"]
env = { RUSTDOCFLAGS = "-D warnings" }
command = "cargo"
args = ["doc", "--no-deps", "--all-features", "--locked"]

[tasks."test:audit"]
install_crate = { crate_name = "cargo-audit", binary = "cargo-audit", test_arg = "--version" }
command = "cargo"
args = ["audit"]

[tasks."test:deny"]
install_crate = { crate_name = "cargo-deny", binary = "cargo-deny", test_arg = "--version", version = "0.19.6" }
command = "cargo"
args = ["deny", "check"]

[tasks."test:vet"]
install_crate = { crate_name = "cargo-vet", binary = "cargo-vet", test_arg = "--version", version = "0.10.0" }
command = "cargo"
args = ["vet", "--locked"]

[tasks."test:typos"]
install_crate = { crate_name = "typos-cli", binary = "typos", test_arg = "--version", version = "1.42.3" }
cwd = ".."
command = "typos"

[tasks."test:machete"]
install_crate = { crate_name = "cargo-machete", binary = "cargo-machete", test_arg = "--version", version = "0.9.1" }
command = "cargo"
args = ["machete"]

[tasks."test:coverage"]
install_crate = { crate_name = "cargo-llvm-cov", binary = "cargo-llvm-cov", test_arg = "--version", version = "0.6.21" }
command = "cargo"
args = ["llvm-cov", "--all-features", "--locked"]

[tasks."test:msrv"]
script_runner = "@shell"
script = 'cargo +1.85.0 check --all-targets --locked'

[tasks.test]
description = "Run every check: lint + unit + docs + audit + deny + vet + typos + machete + coverage + msrv"
dependencies = [
    "test:lint", "test:unit", "test:docs", "test:audit", "test:deny",
    "test:vet", "test:typos", "test:machete", "test:coverage", "test:msrv",
]

[tasks.default]
alias = "test"
```

`install_crate = { ..., version = "X" }` makes `cargo-make` install a
pinned version of each tool on demand if it's missing — CI doesn't need a
separate "install tool" step per job beyond `taiki-e/install-action` (see
`12-ci-cd-and-repo-docs.md`).

### Root `Makefile` (thin wrapper so `make cli:test` etc. work from repo root)

```makefile
.DEFAULT_GOAL := cli:run

cli\:run:
	cd cli && cargo run -q -- $(filter-out $@,$(MAKECMDGOALS))

cli\:lint:
	cd cli && cargo make test:lint

cli\:test:
	cd cli && cargo make test

...
```

(See `12-ci-cd-and-repo-docs.md` for the full file, including `skills:lint`
and `evals:test` targets — it's the entry point CI actually calls.)

## Implementation notes for this repo

- Adding just the `[lints]` block to the existing flat `Cargo.toml` (no
  `[workspace]` needed if this repo stays single-crate) will surface a
  batch of `clippy::pedantic` warnings and any `unwrap()`/`println!`
  currently in `src/*.rs` — expect to need `04-update-command.md`-style
  `out: &mut impl Write` refactors to satisfy `print_stdout`/`print_stderr`
  if the current code uses `println!` directly.
- `cargo-make` is optional tooling, not a language feature — can be
  adopted independently of the lint changes, or skipped in favor of plain
  `cargo` commands / a simpler `Makefile` if this repo doesn't want the
  `cargo-make` dependency.
- The `[profile.release]` block (`lto = "fat"`, `codegen-units = 1`,
  `panic = "abort"`, `strip = "symbols"`) is free to adopt regardless of
  everything else — it only affects `cargo build --release` binary size/
  speed, no code changes required. Note `panic = "abort"` changes behavior
  if any code currently relies on unwinding (e.g. catching a panic) — grep
  for `catch_unwind` before adopting.
