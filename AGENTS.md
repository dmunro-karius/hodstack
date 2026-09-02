# AGENTS.md

This file is for a coding agent working on this repository. Users do not
read it. Public text — `README.md`, release notes, the crate
`description` — follows a separate voice: calm, concrete, no superlatives,
no unverified claims. At most one dry joke per page. Do not explain the
joke.

## 1. Style for this file and for rule files

Write in [ASD-STE100 Simplified Technical English](https://www.asd-ste100.org).
One instruction per sentence. Imperative mood. Active voice. One meaning
per word. No contractions. No synonym variation — see `CONTEXT.md` for the
one word this repository uses for each concept.

Use GitHub-flavored Markdown. One `#` heading per file. One paragraph per
line, so a diff of this file stays small. Number sections so other files
can cross-reference them ("see section 3").

## 2. When to add a rule

A rule records a fault that already happened. Name the decision that was
made wrong, or name the decision that these files failed to answer. A
change you can only imagine going wrong is not a fault; do not write a
rule for it.

A correction from a user is a rule. It needs no further justification.

When a rule becomes obsolete, replace it in place. Do not leave it beside
its replacement with a note that it is gone — git history is the
changelog, not this file.

## 3. Code style

Write no comment. Delete each comment you find in a file you change, if
the code around it still says what the comment said. Express intent
through naming and types instead — give each value a type that makes a
wrong value impossible.

Write no sentence in this file, or in any doc, for a fact an agent finds
by reading the code. Put the fact in the code and in a test. A test stops
the agent that breaks the fact; a sentence here does not.

Match `rustfmt.toml` and the lints in `Cargo.toml`'s `[lints]` table. Run
`cargo make test:lint` before you call a change finished.

## 4. Scope

Do not add a feature, an abstraction, or error handling for a case that
cannot happen. Three similar lines beat one premature abstraction. See
`docs/README.md` for the plan this repository is working through; do not
pull work forward from a later doc into an earlier one's change.
