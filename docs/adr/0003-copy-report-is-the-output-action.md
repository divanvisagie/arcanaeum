# 0003. Copy Report is the title bar's output action

Date: 2026-10-15

## Status

Proposed

## Context

Patina's title bar puts the actions that make the app's main output on the right, next to the
window buttons (Patina's ADR-0006). Arcanaeum's output is its answer for a save: which plugins
it needs, which are installed, and where to get the rest. The old app had no way to take that
answer anywhere; it could only be read off the screen.

## Decision

We will put one button on the right of the title bar, Copy Report (also Edit > Copy Report,
Ctrl+Shift+C), which copies the chosen save's character, file and every plugin with its status
and download links as tab-separated text. It's disabled until a save is open.

## Consequences

- The report can be pasted into a note, a chat or a mod manager's search.
- If Arcanaeum later downloads or installs mods (the README's future plans), that action would
  be the output and belong on the right instead; this ADR would then be superseded.
