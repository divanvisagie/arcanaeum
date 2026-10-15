# 0006. Switch plugins in memory, write Plugins.txt on Apply

Date: 2026-10-15

## Status

Proposed

## Context

With skyplug-core (ADR-0005), Arcanaeum can switch plugins on and off in `Plugins.txt`,
which decides what the game loads. A wrong write breaks the user's game, and Skyrim itself
rewrites `Plugins.txt` when it quits, undoing anything written while it ran. skyplug's
terminal UI already settled on holding changes until the user saves them.

## Decision

- A checkbox per plugin that `Plugins.txt` decides (not the base game, Creation Club, or
  plugins that aren't in `Data`), in both the Load Order view and a save's plugin list.
  Switching one only records it; the row is shown in italics, marked "not applied".
- **Apply** (Ctrl+S, also Edit > Apply Changes) writes every change at once through
  skyplug-core's `apply_changes`, which keeps the previous file as `Plugins.txt.bak`. It
  sits on the right of the title bar next to Copy Report, as the app's main output now
  (Patina's ADR-0006), counting the changes waiting. Edit > Revert Changes drops them.
- A save's view offers **Enable Disabled Plugins**: every plugin the save was made with
  that's in `Data` but switched off.
- Apply refuses while Skyrim is running (a `SkyrimSE.exe` process: `/proc` on Linux,
  `tasklist` on Windows), saying why.
- Closing the window with changes not applied asks: Apply and Quit, Quit Without Applying,
  or Cancel.
- The load order shown is skyplug-core's approximation, from `Plugins.txt` as it is on
  disk; pending changes don't reorder it until applied.

## Consequences

- Nothing reaches `Plugins.txt` without an explicit Apply, and the last version is always
  one `.bak` away.
- Changing the game folder drops pending changes, since they belong to the other install.
- Only one backup is kept; a second Apply replaces it.
- Plugins can't be reordered yet, only switched on and off.
