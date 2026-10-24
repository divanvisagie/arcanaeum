# 0008. A tab for each copy of the game

Date: 2026-10-24

## Status

Accepted

## Context

The user has two copies of Skyrim Special Edition: Steam's and GOG's, the second run under umu
in its own Wine prefix (ADR-0007). With one game folder and prefix in the config, moving between
them meant picking both folders again in Settings, and the save folder with them.

Patina already has title bar tabs (its ADR-0009, from Teleprinter): a strip after the menus in
place of the window title, `+` for a new tab, Ctrl+PageUp/PageDown to move between them. A
drop-down of copies in the sidebar was considered; tabs show which copy is open at a glance and
cost nothing to build.

Keeping each tab's whole state (load order, saves, open save, switched plugins) alive at once was
ruled out for now: it would mean a state per tab through all of `App`. A tab starts over when
shown instead; reading a copy again takes a moment.

## Decision

We will keep a list of installs in `config.toml` (`[[installs]]`, each with a name, game folder,
Wine prefix and save folder, and `shown` for the one open), each shown as a tab in the title bar.
Settings edits the shown tab's install, including its name. `+` adds an empty install and opens
Settings for it; closing a tab forgets its install, and the last one stays. A config from before
tabs has its `game_folder` and `save_folder` moved into the first install, and one with no game
folder (Steam's, found each run) is filled in from Steam (ADR-0007).

Plugins switched but not applied belong to the shown copy's `Plugins.txt`, so its tab can't be
left (shown another, closed, or a new one added) until they're applied or reverted; the status
bar says so.

## Consequences

- Moving between Steam's and GOG's copy is one click, each keeping its own folders.
- The window title is gone from the title bar; the tabs take its place.
- Showing a tab reads that copy again, and the open save is closed.
- Switched plugins block switching tabs rather than being kept per tab; revisit if that gets in
  the way.
