# 0002. Read saves on a worker thread

Date: 2026-10-15

## Status

Proposed

## Context

Patina's rules say file I/O never blocks the UI. The old app read every `.ess` in the save folder
in full on the UI thread at start-up, and opening a save also read the Vortex masterlist and
scanned the game's `Data` folder there. A real folder of 95 saves took about five seconds to
scan, during which the window was frozen. The save parser also indexes without bounds checks, so
one truncated file panicked and took the app down.

## Decision

We will do all save reading on one worker thread (`loader.rs`), driven by `Command`s
(`ScanFolder`, `OpenSave`) and answering with `Update`s over a channel, with a repaint request
after each. Each parse runs under `catch_unwind`: a file that panics the parser is counted as
unreadable (shown in the status bar) rather than ending the worker. Answers for a folder or save
that is no longer the current one are dropped.

## Consequences

- The window stays responsive while a folder is scanned; the sidebar and central panel show a
  spinner until the answer arrives.
- One worker means a slow scan delays opening a save until it finishes. Fine for now; a second
  worker or cancellation is the fix if it matters.
- `catch_unwind` hides parser panics behind a count; the parser itself still wants bounds checks.
