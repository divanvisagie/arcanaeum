# 0004. Relicense from MIT to BSD 3-Clause

Date: 2026-10-15

## Status

Accepted

## Context

Arcanaeum has been MIT-licensed since 2022. It is now a Patina app, and Patina is BSD 3-Clause;
the user wanted Arcanaeum to match. Every commit in the history is the user's own (the
"GitHub" committer on some is the web UI merging the user's own pull requests, #1, #3 and #5),
so no other contributor's consent was needed. All dependencies are permissive, or offer a
permissive choice, so none constrains the licence.

## Decision

We will license Arcanaeum under the BSD 3-Clause License from this change on: `LICENSE` holds
its text (copyright 2022-2026, Divan Visagie), and `Cargo.toml` says `BSD-3-Clause`.

## Consequences

- Arcanaeum and Patina share a licence.
- BSD 3-Clause adds one condition to MIT's: the author's name can't be used to endorse
  derived products without permission.
- Copies taken under MIT before the change, including the existing GitHub fork, stay usable
  under MIT; only later versions are BSD 3-Clause. The README says so.
